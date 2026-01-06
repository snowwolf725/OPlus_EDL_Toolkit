import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export function useAdvancedPanelEventHandler(imgSavingPath, isDebug) {

    let xmlContent = ref('<?xml version="1.0" ?>\n<data>\n\t<power DelayInSeconds="0" value="reset" />\n</data >');

    let cmdOutput = ref("");

    let selectedCmd = ref("power");

    const cmdList = ref([{ id: 1, label: 'Read', value: 'read' },
        { id: 1, label: 'Program', value: 'program' },
        { id: 1, label: 'Erase', value: 'erase' },
        { id: 1, label: 'Power', value: 'power' },
        { id: 1, label: 'Sha256init', value: 'sha256init' },
        { id: 1, label: 'Transfercfg', value: 'transfercfg' },
        { id: 1, label: 'Verify', value: 'verify' },
        { id: 1, label: 'Nop', value: 'nop' },
    ]);

    async function runCommand() {
        cmdOutput.value = await invoke("run_command", { cmdType: selectedCmd.value, path: imgSavingPath.value, content: xmlContent.value, isDebug: isDebug.value });
    }

    async function handleSelectCmdChange() {
        if (selectedCmd.value == "read") {
            xmlContent.value = '<?xml version="1.0" ?>\n<data>\n\t<read filename="misc.img" physical_partition_number="0" label="misc" start_sector="8200" num_partition_sectors="256" SECTOR_SIZE_IN_BYTES="4096" sparse="false"/>\n</data>';
        } else if (selectedCmd.value == "program") {
            xmlContent.value = '<?xml version="1.0" ?>\n<data>\n\t<program start_sector="8200" size_in_KB="1024.0" physical_partition_number="0" partofsingleimage="false" file_sector_offset="0" num_partition_sectors="256" readbackverify="false" filename="misc.img" sparse="false" start_byte_hex="0x2008000" SECTOR_SIZE_IN_BYTES="4096" label="misc"/>\n</data>';
        } else if (selectedCmd.value == "erase") {
            xmlContent.value = '<?xml version="1.0" ?>\n<data>\n\t<erase SECTOR_SIZE_IN_BYTES="4096" physical_partition_number="0" start_sector="8200" num_partition_sectors="256"/>\n</data>';
        } else if (selectedCmd.value == "power") {
            xmlContent.value = '<?xml version="1.0" ?>\n<data>\n\t<power DelayInSeconds="0" value="reset" />\n</data>';
        } else if (selectedCmd.value == "sha256init") {
            xmlContent.value = '<?xml version="1.0" ?>\n<data>\n\t<sha256init Verbose="1"/>\n</data>';
        } else if (selectedCmd.value == "transfercfg") {
            xmlContent.value = '<?xml version="1.0" ?>\n<data>\n\t<transfercfg reboot_type="off" timeout_in_sec="90" />\n</data>';
        } else if (selectedCmd.value == "verify") {
            xmlContent.value = '<?xml version="1.0" ?>\n<data>\n\t<verify value="ping" EnableVip="1"/>\n</data>';
        } else if (selectedCmd.value == "nop") {
            xmlContent.value = '<?xml version="1.0" ?>\n<data>\n\t<nop verbose="0" value="ping"/>\n</data>';
        }
    }

    return {
        xmlContent,
        cmdOutput,
        selectedCmd,
        cmdList,
        runCommand,
        handleSelectCmdChange,
    }
}