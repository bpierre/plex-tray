use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use ksni::{
    MenuItem, TrayMethods,
    menu::{CheckmarkItem, StandardItem},
};
use tokio::{process::Command, sync::mpsc};
use zbus::{Connection, Proxy, fdo::RequestNameFlags, zvariant::OwnedObjectPath};

mod settings;

const APP: &str = "io.github.bpierre.PlexTray";
const APP_PATH: &str = "/io/github/bpierre/PlexTray";
const SYSTEMD: &str = "org.freedesktop.systemd1";
const UNIT_INTERFACE: &str = "org.freedesktop.systemd1.Unit";
const UNIT: &str = "plexmediaserver.service";
const WEB: &str = "http://127.0.0.1:32400/web";

enum Event {
    Activate,
    Open,
    Start,
    Stop,
    Quit,
    SetMonochrome(bool),
    Error(String),
    Completed {
        quit: bool,
        result: Result<(), String>,
    },
}

struct Controller(mpsc::UnboundedSender<Event>);

#[zbus::interface(name = "io.github.bpierre.PlexTray")]
impl Controller {
    fn activate(&self) {
        let _ = self.0.send(Event::Activate);
    }
    fn start(&self) {
        let _ = self.0.send(Event::Start);
    }
    fn stop(&self) {
        let _ = self.0.send(Event::Stop);
    }
    fn quit(&self) {
        let _ = self.0.send(Event::Quit);
    }
    fn set_monochrome(&self, enabled: bool) {
        let _ = self.0.send(Event::SetMonochrome(enabled));
    }
}

struct PlexTray {
    state: String,
    pending: Option<&'static str>,
    error: Option<String>,
    monochrome: bool,
    sender: mpsc::UnboundedSender<Event>,
}

impl PlexTray {
    fn status_text(&self) -> String {
        let status = self.pending.unwrap_or(match self.state.as_str() {
            "active" => "running",
            "inactive" => "stopped",
            "activating" => "starting…",
            "deactivating" => "stopping…",
            state => state,
        });
        format!("Plex: {status}")
    }

    fn item(&self, label: &str, event: fn() -> Event, enabled: bool) -> MenuItem<Self> {
        StandardItem {
            label: label.into(),
            enabled,
            activate: Box::new(move |tray: &mut Self| {
                let _ = tray.sender.send(event());
            }),
            ..Default::default()
        }
        .into()
    }
}

impl ksni::Tray for PlexTray {
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        "plex-tray".into()
    }
    fn title(&self) -> String {
        "Plex Media Server".into()
    }

    fn icon_name(&self) -> String {
        if self.monochrome {
            "plex-tray-symbolic".into()
        } else {
            String::new()
        }
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let color = if self.monochrome {
            // Fallback for hosts that cannot resolve the theme's symbolic icon.
            [190, 190, 190]
        } else if self.error.is_some() || self.state == "failed" {
            [224, 90, 90]
        } else if self.state == "active" && self.pending.is_none() {
            [229, 160, 13]
        } else {
            [136, 136, 136]
        };
        // Draw the same chevron as the desktop SVG, in network-order ARGB.
        // Shipping pixels over D-Bus keeps the tray independent of asset paths.
        let mut data = vec![0; 48 * 48 * 4];
        for y in 0..48 {
            let py = (y as f32 + 0.5) / 2.0;
            for x in 0..48 {
                let px = (x as f32 + 0.5) / 2.0;
                let left = 12.0 - (py - 12.0).abs() * 0.7;
                if (2.0..22.0).contains(&py) && (left..left + 7.0).contains(&px) {
                    let offset = (y * 48 + x) * 4;
                    data[offset..offset + 4].copy_from_slice(&[255, color[0], color[1], color[2]]);
                }
            }
        }
        vec![ksni::Icon {
            width: 48,
            height: 48,
            data,
        }]
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: self.status_text(),
            description: self
                .error
                .as_ref()
                .map(|s| escape_markup(s))
                .unwrap_or_default(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let idle = self.pending.is_none();
        let mut items = vec![
            StandardItem {
                label: self.status_text(),
                enabled: false,
                ..Default::default()
            }
            .into(),
        ];
        if let Some(error) = &self.error {
            items.push(
                StandardItem {
                    label: format!("Error: {}", error.lines().next().unwrap_or(error)),
                    enabled: false,
                    ..Default::default()
                }
                .into(),
            );
        }
        items.extend([
            MenuItem::Separator,
            self.item("Open Plex", || Event::Open, idle && self.state == "active"),
            self.item(
                "Start Plex",
                || Event::Start,
                idle && matches!(self.state.as_str(), "inactive" | "failed"),
            ),
            self.item(
                "Stop Plex",
                || Event::Stop,
                idle && self.state != "inactive",
            ),
            MenuItem::Separator,
            CheckmarkItem {
                label: "Monochrome icon".into(),
                checked: self.monochrome,
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.sender.send(Event::SetMonochrome(!tray.monochrome));
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            self.item("Quit Plex", || Event::Quit, idle),
        ]);
        items
    }
}

fn escape_markup(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

async fn service_state(properties: &zbus::fdo::PropertiesProxy<'_>) -> Result<String> {
    let value = properties
        .get(UNIT_INTERFACE.try_into()?, "ActiveState")
        .await?;
    Ok(String::try_from(value)?)
}

fn operate(verb: &'static str, quit: bool, sender: mpsc::UnboundedSender<Event>) {
    tokio::spawn(async move {
        let result = async {
            let output = Command::new("/usr/bin/systemctl")
                .args(["--no-ask-password", verb, UNIT])
                .output()
                .await?;
            if !output.status.success() {
                bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
            }
            Ok::<(), anyhow::Error>(())
        }
        .await
        .map_err(|error| error.to_string());
        let _ = sender.send(Event::Completed { quit, result });
    });
}

fn open_web(sender: mpsc::UnboundedSender<Event>) {
    tokio::spawn(async move {
        let result = async {
            let output = Command::new("xdg-open").arg(WEB).output().await?;
            if !output.status.success() {
                bail!(
                    "Could not open Plex: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                );
            }
            Ok::<(), anyhow::Error>(())
        }
        .await;
        if let Err(error) = result {
            let _ = sender.send(Event::Error(error.to_string()));
        }
    });
}

fn report_error(message: &str) {
    eprintln!("plex-tray: {message}");
    let message = message.to_owned();
    tokio::spawn(async move {
        let _ = Command::new("notify-send")
            .args([
                "--app-name=Plex Media Server",
                "--icon=dialog-error",
                "Plex Tray error",
                &escape_markup(&message),
            ])
            .status()
            .await;
    });
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let mut no_start = false;
    let mut icon_override = None;
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--no-start" => no_start = true,
            "--monochrome" => icon_override = Some(true),
            "--color" => icon_override = Some(false),
            "--version" | "-V" => {
                println!("plex-tray {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "-h" => {
                println!(
                    "Usage: plex-tray [OPTIONS]\n\nStart Plex and show its tray controls.\n\nOptions:\n  --no-start     Show status without starting the service\n  --monochrome   Use a theme-aware monochrome icon\n  --color        Use colored service status icons\n  -h, --help     Show this help\n  -V, --version  Show the version"
                );
                return Ok(());
            }
            _ => bail!("Unknown argument: {argument}"),
        }
    }

    let (sender, mut events) = mpsc::unbounded_channel();
    let session = Connection::session()
        .await
        .context("Connect to desktop D-Bus")?;
    session
        .object_server()
        .at(APP_PATH, Controller(sender.clone()))
        .await?;
    match session
        .request_name_with_flags(APP, RequestNameFlags::DoNotQueue.into())
        .await
    {
        Ok(_) => {}
        Err(zbus::Error::NameTaken) => {
            let proxy = Proxy::new(&session, APP, APP_PATH, APP).await?;
            if let Some(enabled) = icon_override {
                proxy.call::<_, _, ()>("SetMonochrome", &(enabled,)).await?;
            }
            if !no_start {
                proxy.call::<_, _, ()>("Activate", &()).await?;
            }
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    }

    let settings_path = settings::path();
    let saved = match settings::load(settings_path.as_deref()) {
        Ok(saved) => saved,
        Err(error) => {
            eprintln!("plex-tray: could not load settings: {error}");
            settings::Settings::default()
        }
    };
    let system = Connection::system()
        .await
        .context("Connect to system D-Bus")?;
    let manager = Proxy::new(
        &system,
        SYSTEMD,
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    )
    .await?;
    manager.call::<_, _, ()>("Subscribe", &()).await?;
    let mut units = manager.receive_signal("UnitNew").await?;
    let path: OwnedObjectPath = manager.call("LoadUnit", &(UNIT,)).await?;
    let properties = zbus::fdo::PropertiesProxy::builder(&system)
        .destination(SYSTEMD)?
        .path(path)?
        .build()
        .await?;
    let mut changes = properties.receive_properties_changed().await?;
    let mut state = service_state(&properties).await?;
    let tray = PlexTray {
        state: state.clone(),
        pending: None,
        error: None,
        monochrome: icon_override.unwrap_or(saved.monochrome),
        sender: sender.clone(),
    }
    .spawn()
    .await
    .context("Register StatusNotifier tray")?;
    let mut busy = false;
    if let Some(enabled) = icon_override {
        sender.send(Event::SetMonochrome(enabled))?;
    }
    if !no_start && !matches!(state.as_str(), "active" | "activating") {
        sender.send(Event::Start)?;
    }
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    loop {
        tokio::select! {
            signal = changes.next() => {
                let Some(signal) = signal else { bail!("System D-Bus disconnected"); };
                let args = signal.args()?;
                if args.interface_name().as_str() == UNIT_INTERFACE {
                    if let Some(value) = args.changed_properties().get("ActiveState") {
                        state = value.downcast_ref::<&str>()?.to_owned();
                    } else if args.invalidated_properties().contains(&"ActiveState") {
                        state = service_state(&properties).await?;
                    }
                    tray.update(|tray| tray.state = state.clone()).await;
                }
            }
            signal = units.next() => {
                let Some(signal) = signal else { bail!("System D-Bus disconnected"); };
                let (name, _path): (String, OwnedObjectPath) = signal.body().deserialize()?;
                if name == UNIT {
                    state = service_state(&properties).await?;
                    tray.update(|tray| tray.state = state.clone()).await;
                }
            }
            Some(event) = events.recv() => {
                match event {
                    Event::Completed { quit, result } => {
                        busy = false;
                        match result {
                            Ok(()) if quit => break,
                            Ok(()) => { tray.update(|tray| tray.pending = None).await; }
                            Err(error) => {
                                report_error(&error);
                                tray.update(|tray| { tray.pending = None; tray.error = Some(error); }).await;
                            }
                        }
                    }
                    Event::Error(error) => {
                        report_error(&error);
                        tray.update(|tray| tray.error = Some(error)).await;
                    }
                    Event::SetMonochrome(enabled) => {
                        tray.update(|tray| tray.monochrome = enabled).await;
                        if let Err(error) = settings::save(settings_path.as_deref(), settings::Settings { monochrome: enabled }) {
                            let message = format!("Could not save icon preference: {error}");
                            report_error(&message);
                            tray.update(|tray| tray.error = Some(message)).await;
                        }
                    }
                    _ if busy => {}
                    Event::Open => open_web(sender.clone()),
                    Event::Activate if state == "active" => open_web(sender.clone()),
                    Event::Activate | Event::Start | Event::Stop | Event::Quit => {
                        let quit = matches!(event, Event::Quit);
                        let start = matches!(event, Event::Activate | Event::Start);
                        let verb = if start { "start" } else { "stop" };
                        busy = true;
                        tray.update(|tray| {
                            tray.pending = Some(if start { "starting…" } else { "stopping…" });
                            tray.error = None;
                        }).await;
                        operate(verb, quit, sender.clone());
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => break,
            _ = terminate.recv() => break,
        }
    }
    tray.shutdown().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ksni::Tray;

    #[test]
    fn monochrome_has_a_symbolic_name_and_neutral_fallback_in_every_state() {
        let (sender, _receiver) = mpsc::unbounded_channel();
        for state in ["active", "inactive", "failed"] {
            let tray = PlexTray {
                state: state.into(),
                pending: None,
                error: None,
                monochrome: true,
                sender: sender.clone(),
            };
            assert_eq!(tray.icon_name(), "plex-tray-symbolic");
            let icons = tray.icon_pixmap();
            let visible: Vec<_> = icons[0]
                .data
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|pixel| pixel[0] != 0)
                .collect();
            assert!(!visible.is_empty());
            assert!(
                visible
                    .iter()
                    .all(|pixel| pixel[1] == pixel[2] && pixel[2] == pixel[3])
            );
        }
    }
}
