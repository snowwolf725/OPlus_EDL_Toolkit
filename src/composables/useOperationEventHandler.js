import { XMLBuilder } from 'fast-xml-parser';
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
export function useOperationEventHandler(imgSavingPath, isBuildIn, isDialogOpen, tableData, isDebug, t) {

    async function erasePart() {
        const builder = new XMLBuilder({
            ignoreAttributes: false,
            format: true,
        });
        let parts = [];
        const data = {
            erase: parts
        };
        const jsObj = {
            "?xml": {
                "@_version": "1.0"
            },
            data: data
        };
        tableData.value.forEach((item, index) => {
            if (item.chk) {
                const num = item.lun;
                const partname = item.partName;
                let part_start_sector = item.partStart;

                parts.push({
                    "@_SECTOR_SIZE_IN_BYTES": "4096",
                    "@_label": partname,
                    "@_physical_partition_number": num,
                    "@_start_sector": part_start_sector,
                    "@_num_partition_sectors": part_num,
                });
            }
        });
        const xmlContent = builder.build(jsObj);
        await invoke("erase_part", { xml: xmlContent, isDebug: isDebug.value });
    }

    async function readDeviceInfo() {
        let result = await invoke("read_device_info", { isDebug: isDebug.value });
        alert(result);
    }

    async function readGPT() {
        await invoke("read_gpt", { isDebug: isDebug.value });
    }

    async function readPart() {
        const builder = new XMLBuilder({
            ignoreAttributes: false,
            format: true,
        });
        let reads = [];
        const data = {
            read: reads
        };
        const jsObj = {
            "?xml": {
                "@_version": "1.0"
            },
            data: data
        };
        tableData.value.forEach((item, index) => {
            if (item.chk) {
                const num = item.lun;
                const partname = item.partName;
                let part_size = item.partSize;
                let part_start_sector = item.partStart;
                const part_num = item.partNum;

                if (part_size.length >= 2) {
                    part_size = part_size.slice(0, -2);
                }

                reads.push({
                    "@_filename": partname + ".img",
                    "@_physical_partition_number": num,
                    "@_label": partname,
                    "@_start_sector": part_start_sector,
                    "@_num_partition_sectors": part_num,
                    "@_SECTOR_SIZE_IN_BYTES": "4096",
                    "@_sparse": "false"
                });
            }
        });
        const xmlContent = builder.build(jsObj);
        await invoke("read_part", { xml: xmlContent, folder: imgSavingPath.value, isDebug: isDebug.value });
    }

    async function saveToXML() {
        const builder = new XMLBuilder({
            ignoreAttributes: false,
            format: true,
            suppressBooleanAttributes: false,
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
        let count = 0;
        tableData.value.forEach((item, index) => {
            if (item.chk) {
                count++;
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
                    part_start_sector = parseInt(part_start_sector) * 4096;
                    start_byte_hex = '0x' + part_start_sector.toString(16);
                }

                programs.push({
                    "@_start_sector": part_start_sector,
                    "@_size_in_KB": part_size,
                    "@_physical_partition_number": num,
                    "@_partofsingleimage": "false",
                    "@_file_sector_offset": "0",
                    "@_num_partition_sectors": part_num,
                    "@_readbackverify": "false",
                    "@_filename": item.imgPath,
                    "@_sparse": item.sparse,
                    "@_start_byte_hex": start_byte_hex,
                    "@_SECTOR_SIZE_IN_BYTES": "4096",
                    "@_label": partname
                });
            }
        });
        if (count > 0) {
            const xmlContent = builder.build(jsObj);
            const path = await save({ filters: [{ name: 'XML file', extensions: ['xml'] }] });
            if (path != null) {
                await invoke("save_to_xml", { path: path, xml: xmlContent });
            }
        } else {
            alert(t('operation.saveAlert'));
        }
    }

    async function sendLoader() {
        let loader = document.getElementById('loaderPathDisplay').value;
        let digest = document.getElementById('digestPathDisplay').value;
        let sig = document.getElementById('signPathDisplay').value;

        await invoke("send_loader", { loader: loader, digest: digest, sig: sig, native: isBuildIn.value, isDebug: isDebug.value });
    }

    async function switchSlot(slot) {
        isDialogOpen.value = false;
        await invoke("switch_slot", { slot: slot, isDebug: isDebug.value });
    }

    async function writeFromXML() {
        try {
            const file = await open({
                multiple: false,
                directory: false,
                filters: [{ name: 'XML file', extensions: ['xml'] }],
            });
            if (file) {
                await invoke("write_from_xml", { file_path: file, isDebug: isDebug.value });
            }
        } catch (error) {
            console.error('Error occurred while selecting a file:', error);
        }
    }

    async function writePart() {
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
        tableData.value.forEach((item, index) => {
            if (item.chk) {
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
                    "@_filename": item.imgPath,
                    "@_sparse": item.sparse,
                    "@_start_byte_hex": start_byte_hex,
                    "@_SECTOR_SIZE_IN_BYTES": "4096",
                    "@_label": partname
                });
            }
        });
        const xmlContent = builder.build(jsObj);
        await invoke("write_part", { xml: xmlContent, isDebug: isDebug.value });
    }

    return {
        erasePart,
        readDeviceInfo,
        readGPT,
        readPart,
        saveToXML,
        sendLoader,
        switchSlot,
        writeFromXML,
        writePart,
    }
}
