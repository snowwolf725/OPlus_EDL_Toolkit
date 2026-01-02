import { XMLBuilder } from 'fast-xml-parser';
import { invoke } from "@tauri-apps/api/core";


export function useRebootPanelEventHandler(tableData, isDebug, t) {

    async function rebootToEdl() {
        await invoke("reboot_to_edl", { isDebug: isDebug.value });
    }

    async function rebootToFastboot() {
        const builder = new XMLBuilder({
            ignoreAttributes: false,
            format: true,
            suppressBooleanAttributes: false,
            suppressEmptyNode: true,
        });
        let programs = [];
        const data = {
            program: programs
        };
        const jsObj = {
            "?xml": {
                "@_version": "1.0"
            },
            data: data
        };
        let isFound = false;
        tableData.value.forEach((item, index) => {
            if (item.partName == "misc") {
                isFound = true;
                const num = item.lun;
                const partname = item.partName;
                let part_size = item.partSize;
                let part_start_sector = item.partStart;
                const part_num = item.partNum;

                if (part_size.length >= 2) {
                    part_size = part_size.slice(0, -2);
                }
                let start_byte_hex = "";
                if (isNaN(num) == false) {
                    start_byte_hex = parseInt(part_start_sector) * 4096;
                    start_byte_hex = '0x' + start_byte_hex.toString(16);
                }

                programs.push({
                    "@_start_sector": part_start_sector,
                    "@_size_in_KB": part_size,
                    "@_physical_partition_number": num,
                    "@_partofsingleimage": "false",
                    "@_file_sector_offset": "0",
                    "@_num_partition_sectors": part_num,
                    "@_readbackverify": "false",
                    "@_filename": "misc_tofastbootd.img",
                    "@_sparse": item.sparse,
                    "@_start_byte_hex": start_byte_hex,
                    "@_SECTOR_SIZE_IN_BYTES": "4096",
                    "@_label": partname
                });
            }
        });
        if (isFound) {
            const xmlContent = builder.build(jsObj);
            await invoke("reboot_to_fastboot", { xml: xmlContent, isDebug: isDebug.value });
        } else {
            alert(t('reboot.miscNotFound'));
        }
    }

    async function rebootToRecovery() {
        const builder = new XMLBuilder({
            ignoreAttributes: false,
            format: true,
            suppressBooleanAttributes: false,
            suppressEmptyNode: true,
        });
        let programs = [];
        const data = {
            program: programs
        };
        const jsObj = {
            "?xml": {
                "@_version": "1.0"
            },
            data: data
        };
        let isFound = false;
        tableData.value.forEach((item, index) => {
            if (item.partName == "misc") {
                isFound = true;
                const num = item.lun;
                const partname = item.partName;
                let part_size = item.partSize;
                let part_start_sector = item.partStart;
                const part_num = item.partNum;

                if (part_size.length >= 2) {
                    part_size = part_size.slice(0, -2);
                }
                let start_byte_hex = "";
                if (isNaN(num) == false) {
                    start_byte_hex = parseInt(part_start_sector) * 4096;
                    start_byte_hex = '0x' + start_byte_hex.toString(16);
                }

                programs.push({
                    "@_start_sector": part_start_sector,
                    "@_size_in_KB": part_size,
                    "@_physical_partition_number": num,
                    "@_partofsingleimage": "false",
                    "@_file_sector_offset": "0",
                    "@_num_partition_sectors": part_num,
                    "@_readbackverify": "false",
                    "@_filename": "misc_torecovery.img",
                    "@_sparse": item.sparse,
                    "@_start_byte_hex": start_byte_hex,
                    "@_SECTOR_SIZE_IN_BYTES": "4096",
                    "@_label": partname
                });
            }
        });
        if (isFound) {
            const xmlContent = builder.build(jsObj);
            await invoke("reboot_to_recovery", { xml: xmlContent, isDebug: isDebug.value });
        } else {
            alert(t('reboot.miscNotFound'));
        }
    }

    async function rebootToSystem() {
        await invoke("reboot_to_system", { isDebug: isDebug.value });
    }

    return {
        rebootToEdl,
        rebootToFastboot,
        rebootToRecovery,
        rebootToSystem,
    }
}
