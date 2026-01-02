import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { readTextFile, BaseDirectory } from '@tauri-apps/plugin-fs';
import { XMLParser } from 'fast-xml-parser';

export function useConfigPanelEventHandler(tableData, activeTab, activeStep) {

    async function btn_selectLoaderFileClick() {
        try {
            const file = await open({
                multiple: false,
                directory: false,
            });
            if (file) {
                document.getElementById('loaderPathDisplay').value = file;
                await invoke("identify_loader", { path: file });
            }
        } catch (error) {
            console.error('Error occurred while selecting a file:', error);
        }
    }

    async function btn_selectDigestFileClick() {
        try {
            const file = await open({
                multiple: false,
                directory: false,
            });
            if (file) {
                document.getElementById('digestPathDisplay').value = file;
            }
        } catch (error) {
            console.error('Error occurred while selecting a file:', error);
        }
    }

    async function btn_selectSignFileClick() {
        try {
            const file = await open({
                multiple: false,
                directory: false,
            });
            if (file) {
                document.getElementById('signPathDisplay').value = file;
            }
        } catch (error) {
            console.error('Error occurred while selecting a file:', error);
        }
    }

    async function btn_selectRawXmlFileClick() {
        try {
            const file = await open({
                multiple: false,
                directory: false,
                filters: [{ name: 'XML file', extensions: ['xml'] }],
            });
            if (file) {
                document.getElementById('rawXmlPathDisplay').value = file;
                const partTable = document.getElementById('partTable');
                tableData.value = [];

                const content = await readTextFile(file, {
                    baseDir: BaseDirectory.AppConfig,
                });

                const options = {
                    ignoreAttributes: false,
                    attributeNamePrefix: "@_"
                };
                const parser = new XMLParser(options);
                let xmlObj = parser.parse(content);
                if (xmlObj !== null && xmlObj.data !== null && xmlObj.data.program !== null && xmlObj.data.program.length != 0) {
                    let i = 0;
                    for (let item of xmlObj.data.program) {
                        tableData.value.push({
                            chk: false,
                            lun: item['@_physical_partition_number'],
                            partName: item['@_label'],
                            partSize: item['@_size_in_KB'] + "KB",
                            partStart: item['@_start_sector'],
                            partNum: item['@_num_partition_sectors'],
                            imgPath: '',
                            sel: '',
                            sparse: item['@_sparse'],
                        });
                        i++;
                    }
                } else {
                    alert('XML fie invalid');
                }

            }
        } catch (error) {
            console.error('Error occurred while selecting a file:', error);
        }
    }

    async function btn_selectEdlFolderClick() {
        try {
            const dir = await open({
                multiple: false,
                directory: true,
            });
            if (dir) {
                document.getElementById('edlFolderPathDisplay').value = dir;
                activeTab.value = 'tab_edl';
                activeStep.value = 2;
            }
        } catch (error) {
            console.error('Error occurred while selecting a folder:', error);
        }
    }

    return {
        btn_selectLoaderFileClick,
        btn_selectDigestFileClick,
        btn_selectSignFileClick,
        btn_selectRawXmlFileClick,
        btn_selectEdlFolderClick,
    }
}