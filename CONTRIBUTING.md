# Contributing

The application currently targets Arch Linux desktops with systemd and a
StatusNotifier tray.
Keep the controller unprivileged and keep authorization limited to the existing
Plex service's start and stop operations.

Run the checks listed in the README before submitting a change. Rust tests
cover preference parsing/persistence and CLI behavior without a running desktop
or service. JavaScript tests evaluate the Polkit rule with representative
subjects and actions; they do not change system authorization.

For changes affecting the tray or service control, also check on a desktop:

1. Launch the tray with Plex stopped, and confirm the service starts.
2. Stop and start it from the menu; check status and available menu actions.
3. Change the icon style, relaunch, and confirm the preference is retained.
4. Launch a second instance and confirm there is only one tray icon.
5. Quit Plex and confirm both the service and tray stop.
6. For authorization changes, check that removing the package revokes the
   permission while leaving the separate Plex package installed.

Use `--no-start` when inspecting the tray without changing the service state.
Do not put credentials, local build output, or private system configuration in
the repository. Runtime assets and metadata should work without home-directory
paths. Keep Cargo.lock committed for the application.
