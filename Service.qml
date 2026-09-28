import QtQuick
import Quickshell.Io

Item {
  Process {
    id: installBindings
    command: ["bash", "/home/jff/.config/omarchy/plugins/omarchy-app-keys/install-hypr.sh"]
    Component.onCompleted: running = true
  }
}
