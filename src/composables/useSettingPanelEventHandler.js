import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export function useSettingPanelEventHandler(portName, isSentLoader) {

    let imgSavingPath = ref("img/");
    let isBuildIn = ref(false);
    let isProtectLun5 = ref(true);
    let isEnablePing = ref(true);
    let isDebug = ref(false);
    
    async function changeSavingPath() {
        try {
            const dir = await open({
                multiple: false,
                directory: true,
            });
            if (dir) {
                imgSavingPath.value = dir;
            }
        } catch (error) {
            console.error('Error occurred while selecting a folder:', error);
        }
    }

    async function sendPing() {
        if (portName.value == "N/A") {
            isSentLoader = false;
        }

        if (isEnablePing.value && isSentLoader && isCommandRunning == false) {
            await invoke("send_ping", { isDebug: isDebug.value });
        }
    }

    return {
        imgSavingPath,
        isBuildIn,
        isProtectLun5,
        isEnablePing,
        isDebug,
        changeSavingPath,
        sendPing,
    }
}
