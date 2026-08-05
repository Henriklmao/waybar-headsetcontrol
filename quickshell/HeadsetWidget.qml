import Quickshell
import Quickshell.Io
import QtQuick

Scope {
    property string headsetProduct: ""
    property int batteryLevel: -1
    property bool charging: false
    property string headsetIcon: "󰋎"
    property bool connected: false

    Timer {
        interval: 10000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: process.running = true
    }

    Process {
        id: process
        command: ["headset-tui", "--quickshell-status"]
        stdout: SplitParser {
            onRead: data => {
                try {
                    let json = JSON.parse(data.trim());
                    connected = json.connected;
                    headsetProduct = json.product;
                    batteryLevel = json.battery;
                    charging = json.charging;
                    headsetIcon = json.icon;
                } catch (e) {
                    connected = false;
                }
            }
        }
    }

    Component {
        id: headsetWidgetComponent

        Rectangle {
            id: container
            width: visible ? implicitWidth + 12 : 0
            height: 24
            visible: connected
            color: "transparent"

            Row {
                anchors.centerIn: parent
                spacing: 4

                Text {
                    text: headsetIcon
                    color: charging ? "#0066ff" : (batteryLevel > 50 ? "#90ee90" : (batteryLevel >= 15 ? "#ffff00" : "#ff0000"))
                    font.family: "JetBrainsMono Nerd Font"
                    font.pixelSize: 13
                }

                Text {
                    text: batteryLevel >= 0 ? batteryLevel + "%" : ""
                    color: "#ffffff"
                    font.family: "sans-serif"
                    font.pixelSize: 11
                    anchors.verticalCenter: parent.verticalCenter
                }
            }

            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                onClicked: (mouse) => {
                    if (mouse.button === Qt.RightButton) {
                        let toggleProc = Quickshell.sh("headset-tui --toggle-sidetone");
                    } else {
                        let tuiProc = Quickshell.sh("kitty headset-tui || alacritty -e headset-tui || headset-tui");
                    }
                }
            }

            ToolTip.visible: hovered
            ToolTip.text: connected ? (headsetProduct + " (" + (charging ? "Charging" : batteryLevel + "%") + ")") : "Headset not found"
        }
    }
}
