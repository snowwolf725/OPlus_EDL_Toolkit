import { invoke } from "@tauri-apps/api/core";

export function useEdlPanelEventHandler(isRunning, isProtectLun5, isDebug) {

    async function startFlashing() {
        isRunning.value = true;
        const edlFolder = document.getElementById('edlFolderPathDisplay').value;
        await invoke("start_flashing", { path: edlFolder, isProtectLun5: isProtectLun5.value, isDebug: isDebug.value });
    }

    async function stopFlashing() {
        isRunning.value = false;
        await invoke("stop_flashing");
    }

    return {
        startFlashing,
        stopFlashing,
    }
}
