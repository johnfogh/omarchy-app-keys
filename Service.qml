import QtQuick
import Quickshell.Io

Item {
  Process {
    id: installBindings
    command: ["bash", "/home/jff/.config/omarchy/plugins/jff.app-layer/install-hypr.sh"]
    Component.onCompleted: running = true
  }
}
