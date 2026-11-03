const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const path = require('node:path');
const { test } = require('node:test');
const vm = require('node:vm');

let rule;
vm.runInNewContext(
  readFileSync(path.join(__dirname, '../50-plex-tray.rules'), 'utf8'),
  { polkit: { addRule: (callback) => { rule = callback; }, Result: { YES: 'yes' } } },
  { timeout: 1000 },
);
assert.equal(typeof rule, 'function');

function authorize(details = {}, subject = {}) {
  const action = {
    id: 'org.freedesktop.systemd1.manage-units',
    unit: 'plexmediaserver.service',
    verb: 'start',
    ...details,
  };
  return rule(
    { id: action.id, lookup: (key) => action[key] },
    { user: 'alice', local: true, active: true, ...subject },
  );
}

test('active local users can start and stop only Plex', () => {
  for (const user of ['alice', 'bob', 'pierre']) {
    for (const verb of ['start', 'stop']) {
      assert.equal(authorize({ verb }, { user }), 'yes');
    }
  }
});

test('remote, inactive, and incomplete subjects are not authorized', () => {
  for (const subject of [
    { local: false }, { active: false }, { local: undefined }, { active: undefined },
  ]) {
    assert.equal(authorize({}, subject), undefined);
  }
});

test('other units and similar names are not authorized', () => {
  for (const unit of ['sshd.service', 'plexmediaserver', 'plexmediaserver.service.evil', undefined]) {
    assert.equal(authorize({ unit }), undefined);
  }
});

test('restart, reload, kill, and property changes are not authorized', () => {
  for (const verb of ['restart', 'reload', 'kill', 'set-property', 'reset-failed', undefined]) {
    assert.equal(authorize({ verb }), undefined);
  }
});

test('other systemd actions are not authorized', () => {
  for (const id of [
    'org.freedesktop.systemd1.manage-unit-files',
    'org.freedesktop.systemd1.reload-daemon',
    'org.freedesktop.systemd1.set-environment',
  ]) {
    assert.equal(authorize({ id }), undefined);
  }
});
