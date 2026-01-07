import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export function useAdvancedPanelEventHandler(imgSavingPath, isDebug) {

    let xmlContent = ref('<?xml version="1.0" ?>\n<data>\n\t<power DelayInSeconds="0" value="reset" />\n</data >');

    let cmdOutput = ref("");

    let selectedCmd = ref("power");

    const cmdList = ref([{ id: 1, label: 'Read', value: 'read' },
        { id: 1, label: 'Program', value: 'program' },
        { id: 2, label: 'Erase', value: 'erase' },
        { id: 3, label: 'Patch', value: 'patch' },
        { id: 4, label: 'Power', value: 'power' },
        { id: 5, label: 'Sha256init', value: 'sha256init' },
        { id: 6, label: 'Transfercfg', value: 'transfercfg' },
        { id: 7, label: 'Verify', value: 'verify' },
        { id: 8, label: 'Nop', value: 'nop' },
        { id: 9, label: 'Getstorageinfo', value: 'getstorageinfo' },
    ]);

    async function runCommand() {
        cmdOutput.value = await invoke("run_command", { cmdType: selectedCmd.value, path: imgSavingPath.value, content: xmlContent.value, isDebug: isDebug.value });
    }

    async function handleSelectCmdChange() {
        if (selectedCmd.value == "read") {
            xmlContent.value = '<?xml version="1.0" ?>\n<data>\n\t<read filename="recovery_a.img" physical_partition_number="4" label="recovery_a" start_sector="375430" num_partition_sectors="25600" SECTOR_SIZE_IN_BYTES="4096" sparse="false"/>\n</data>';
        } else if (selectedCmd.value == "program") {
            xmlContent.value = '<?xml version="1.0" ?>\n<data>\n\t<program start_sector="8200" size_in_KB="1024.0" physical_partition_number="0" partofsingleimage="false" file_sector_offset="0" num_partition_sectors="256" readbackverify="false" filename="misc.img" sparse="false" start_byte_hex="0x2008000" SECTOR_SIZE_IN_BYTES="4096" label="misc"/>\n</data>';
        } else if (selectedCmd.value == "erase") {
            xmlContent.value = '<?xml version="1.0" ?>\n<data>\n\t<erase SECTOR_SIZE_IN_BYTES="4096" physical_partition_number="0" start_sector="8200" num_partition_sectors="256"/>\n</data>';
        } else if (selectedCmd.value == "patch") {
            xmlContent.value = '<?xml version="1.0" ?>\n<patches>\n'
                    + '\t<patch start_sector="2" byte_offset="1704" physical_partition_number="0" size_in_bytes="8" value="NUM_DISK_SECTORS-6." filename="DISK" SECTOR_SIZE_IN_BYTES="4096" what="Update last partition 14 \'userdata\' with actual size in Primary Header."/>\n'
                    + '\t<patch start_sector="NUM_DISK_SECTORS-5." byte_offset="1704" physical_partition_number="0" size_in_bytes="8" value="NUM_DISK_SECTORS-6." filename="DISK" SECTOR_SIZE_IN_BYTES="4096" what="Update last partition 14 \'userdata\' with actual size in Backup Header."/>\n'
                    + '\t<patch start_sector="1" byte_offset="48" physical_partition_number="0" size_in_bytes="8" value="NUM_DISK_SECTORS-6." filename="DISK" SECTOR_SIZE_IN_BYTES="4096" what="Update Primary Header with LastUseableLBA."/>\n'
                    + '\t<patch start_sector="NUM_DISK_SECTORS-1." byte_offset="48" physical_partition_number="0" size_in_bytes="8" value="NUM_DISK_SECTORS-6." filename="DISK" SECTOR_SIZE_IN_BYTES="4096" what="Update Backup Header with LastUseableLBA."/>\n'
                    + '\t<patch start_sector="1" byte_offset="32" physical_partition_number="0" size_in_bytes="8" value="NUM_DISK_SECTORS-1." filename="DISK" SECTOR_SIZE_IN_BYTES="4096" what="Update Primary Header with BackupGPT Header Location."/>\n'
                    + '\t<patch start_sector="NUM_DISK_SECTORS-1." byte_offset="24" physical_partition_number="0" size_in_bytes="8" value="NUM_DISK_SECTORS-1." filename="DISK" SECTOR_SIZE_IN_BYTES="4096" what="Update Backup Header with CurrentLBA."/>\n'
                    + '\t<patch start_sector="NUM_DISK_SECTORS-1" byte_offset="72" physical_partition_number="0" size_in_bytes="8" value="NUM_DISK_SECTORS-5." filename="DISK" SECTOR_SIZE_IN_BYTES="4096" what="Update Backup Header with Partition Array Location."/>\n'
                    + '\t<patch start_sector="1" byte_offset="88" physical_partition_number="0" size_in_bytes="4" value="CRC32(2,4096)" filename="DISK" SECTOR_SIZE_IN_BYTES="4096" what="Update Primary Header with CRC of Partition Array."/>\n'
                    + '\t<patch start_sector="NUM_DISK_SECTORS-1." byte_offset="88" physical_partition_number="0" size_in_bytes="4" value="CRC32(NUM_DISK_SECTORS-5.,4096)" filename="DISK" SECTOR_SIZE_IN_BYTES="4096" what="Update Backup Header with CRC of Partition Array."/>\n'
                    + '\t<patch start_sector="1" byte_offset="16" physical_partition_number="0" size_in_bytes="4" value="0" filename="DISK" SECTOR_SIZE_IN_BYTES="4096" what="Zero Out Header CRC in Primary Header."/>\n'
                    + '\t<patch start_sector="1" byte_offset="16" physical_partition_number="0" size_in_bytes="4" value="CRC32(1,92)" filename="DISK" SECTOR_SIZE_IN_BYTES="4096" what="Update Primary Header with CRC of Primary Header."/>\n'
                    + '\t<patch start_sector="NUM_DISK_SECTORS-1." byte_offset="16" physical_partition_number="0" size_in_bytes="4" value="0" filename="DISK" SECTOR_SIZE_IN_BYTES="4096" what="Zero Out Header CRC in Backup Header."/>\n'
                    + '\t<patch start_sector="NUM_DISK_SECTORS-1." byte_offset="16" physical_partition_number="0" size_in_bytes="4" value="CRC32(NUM_DISK_SECTORS-1.,92)" filename="DISK" SECTOR_SIZE_IN_BYTES="4096" what="Update Backup Header with CRC of Backup Header."/>\n'
                    + '</patches>';
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
        } else if (selectedCmd.value == "getstorageinfo") {
            xmlContent.value = '<?xml version="1.0" ?>\n<data>\n\t<getstorageinfo physical_partition_number="0" />\n</data>';
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