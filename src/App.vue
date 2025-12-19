<script setup>
    import { ref, watch } from "vue";
    import { useI18n } from 'vue-i18n';
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import { open, save } from "@tauri-apps/plugin-dialog";
    import { readTextFile, writeTextFile, BaseDirectory } from '@tauri-apps/plugin-fs';
    import { XMLParser, XMLBuilder } from 'fast-xml-parser';

    const slotDialogRef = ref(null);
    const isDialogOpen = ref(false);
    const isBuildIn = ref(false);
    const activeTab = ref('tab_part');
    const activeStep = ref(1);
    const edl_status = ref(0);
    const percentage = ref(0);

    const { t, locale, availableLocales } = useI18n();

    const tabList = ref([
        { key: 'tab_part', label: t('part.title') },
        { key: 'tab_edl', label: t('edl.title') },
    ]);

    const tableColumns = ref([
        { key: 'chk',       label: ''                    , width: '5%' },
        { key: 'lun',       label: 'LUN'                 , width: '5%' },
        { key: 'partName',  label: t('part.name')        , width: '10%' },
        { key: 'partSize',  label: t('part.size')        , width: '10%' },
        { key: 'partStart', label: t('part.start')       , width: '10%' },
        { key: 'partNum',   label: t('part.num')         , width: '10%' },
        { key: 'imgPath',   label: t('part.imgPath')     , width: '40%' },
        { key: 'sel',       label: t('config.selectBtn') , width: '10%' },
    ]);

    const tableData = ref([]);

    watch(isDialogOpen, (newVal) => {
        const dialog = slotDialogRef.value;
        if (!dialog) return;

        newVal ? dialog.showModal() : dialog.close();
    });

    listen("log_event", (payload) => {
        console.log(payload);
        const logContainer = document.getElementById('logContainer');
        const time = new Date().toLocaleTimeString('zh-CN', { hour12: false });
        const logText = `[${time}] ${payload.payload.toString()}<br>`;
        logContainer.innerHTML += logText;
        logContainer.scrollTop = logContainer.scrollHeight;
    });

    listen("update_partition_table", (payload) => {
        console.log(payload);
        const xml_content = payload.payload.toString();

        const options = {
            ignoreAttributes: false,
            attributeNamePrefix: "@_"
        };
        const parser = new XMLParser(options);
        let xmlObj = parser.parse(xml_content);
        if (xmlObj !== null && xmlObj.data !== null && xmlObj.data.program !== null && xmlObj.data.program.length != 0) {
            let i = 0;
            tableData.value = [];
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
        }
    });

    let greetMsg = ref("");
    let name = ref("");
    let portStatus = ref("EDL device not found");
    let portName = ref("N/A");
    let portNum = ref("");

    async function selectAll() {
        let isFirst = true;
        let state = false;
        tableData.value.forEach((item, index) => {
            if (isFirst) {
                isFirst = false;
                state = !item.chk;
                console.log(state);
            }
            item.chk = state;
        });
    }

    async function startFlashing() {
        //activeStep.value = activeStep.value + 1;
        //percentage.value = 50;
        const edlFolder = document.getElementById('edlFolderPathDisplay').value;
        await invoke("start_flashing", { path: edlFolder });
    }

    async function stopFlashing() {
    }

    const selectImgPath = async(item) => {
        try {
            const file = await open({
                multiple: false,
                directory: false,
            });
            if (file) {
                item.imgPath = file;
            }
        } catch (error) {
            console.error('Error occurred while selecting a file:', error);
        }
    }

    const selectedLang = ref('en');

    const handleSelectLangChange = (e) => {
        locale.value = selectedLang.value;
        tableColumns.value = [
            { key: 'chk', label: '', width: '5%' },
            { key: 'lun', label: 'LUN', width: '5%' },
            { key: 'partName', label: t('part.name'), width: '10%' },
            { key: 'partSize', label: t('part.size'), width: '10%' },
            { key: 'partStart', label: t('part.start'), width: '10%' },
            { key: 'partNum', label: t('part.num'), width: '10%' },
            { key: 'imgPath', label: t('part.imgPath'), width: '40%' },
            { key: 'sel', label: t('config.selectBtn'), width: '10%' },
        ];
        tabList.value = [
            { key: 'tab_part', label: t('part.title') },
            { key: 'tab_edl', label: t('edl.title') },
        ];
    };

    async function rebootToSystem() {
        await invoke("reboot_to_system");
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
            await invoke("reboot_to_recovery", { xml: xmlContent });
        } else {
            alert(t('reboot.miscNotFound'));
        }
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
            await invoke("reboot_to_fastboot", { xml: xmlContent });
        } else {
            alert(t('reboot.miscNotFound'));
        }
    }

    async function rebootToEdl() {
        await invoke("reboot_to_edl");
    }

    async function clearLog() {
        logContainer.innerHTML = "";
    }

    async function writeFromXML() {
        try {
            const file = await open({
                multiple: false,
                directory: false,
                filters: [{ name: 'XML file', extensions: ['xml'] }],
            });
            if (file) {
                await invoke("write_from_xml", { file_path: file });
            }
        } catch (error) {
            console.error('Error occurred while selecting a file:', error);
        }
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
            //await writeTextFile('file.xml', xmlContent, { baseDir: BaseDirectory.AppConfig, });
            const path = await save({ filters: [{ name: 'XML file', extensions: ['xml'] }] });
            if (path != null) {
                await invoke("save_to_xml", { path:path, xml: xmlContent });
            }
        } else {
            alert(t('operation.saveAlert'));
        }
    }

    async function sendLoader() {
        let loader = document.getElementById('loaderPathDisplay').value;
        let digest = document.getElementById('digestPathDisplay').value;
        let sig = document.getElementById('signPathDisplay').value;
        await invoke("send_loader", { loader: loader, digest: digest, sig: sig, native: isBuildIn.value });
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
        await invoke("read_part", { xml: xmlContent });
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
        await invoke("write_part", { xml: xmlContent });
    }

    async function updatePort() {
        const [num, name] = await invoke("update_port");
        portNum.value = num;
        portName.value = name;
        if (portNum.value == "Not found") {
            portStatus.value = t('config.portStatusError');
            portName.value = "N/A";
        } else {
            portStatus.value = t('config.portStatus');
        }
    }

    async function readGPT() {
        await invoke("read_gpt");
    }

    async function readDeviceInfo() {
        let result = await invoke("read_device_info");
        alert(result);
    }

    async function switchSlot(slot) {
        isDialogOpen.value = false;
        await invoke("switch_slot", {slot: slot});
    }

    window.onload = function () {
        invoke("init");

        document.getElementById('btn_selectLoaderFile').addEventListener('click', async () => {
            try {
                const file = await open({
                    multiple: false,
                    directory: false,
                });
                if (file) {
                    document.getElementById('loaderPathDisplay').value = file;
                }
            } catch (error) {
                console.error('Error occurred while selecting a file:', error);
            }
        });
        document.getElementById('btn_selectDigestFile').addEventListener('click', async () => {
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
        });
        document.getElementById('btn_selectSignFile').addEventListener('click', async () => {
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
        });
        document.getElementById('btn_selectRawXmlFile').addEventListener('click', async () => {
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
        });
        document.getElementById('btn_selectEdlFolder').addEventListener('click', async () => {
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
        });

        document.getElementById('partFilter').addEventListener('input', async () => {
            const currentValue = document.getElementById('partFilter').value;
            const allPartNames = document.querySelectorAll('td[class^="partName"]');
            allPartNames.forEach(td => {
                // Check if innerHTML meets the criteria (handling edge cases where innerHTML is null/undefined)
                if (td.innerHTML && td.innerHTML.match(currentValue)) {
                    td.parentElement.style.display = '';
                } else {
                    td.parentElement.style.display = 'none';
                }
            });
        });

    }
    
setInterval(updatePort, 1000);
</script>

<template>
    <dialog ref="slotDialogRef"
            class="slot-dialog">
        <h3 class="dialog-title">选择启动槽位</h3>
        <div class="dialog-btn-group">
            <button class="slot-btn slot-btn-a"
                    @click="switchSlot('A')">
                A
            </button>
            <button class="slot-btn slot-btn-b"
                    @click="switchSlot('B')">
                B
            </button>
        </div>
    </dialog>
    <div class="container">
        <!-- Header -->
        <div class="header">
            <div class="header-left">
                <span>{{ portStatus }}</span>
                <span class="status">{{ portName }}</span>
            </div>
            <select class="header-right" name="language" id="language-select" v-model="selectedLang" @change="handleSelectLangChange">
                <option value="en">English</option>
                <option value="zh_TW">正體中文</option>
                <option value="zh_CN">简体中文</option>
            </select>
        </div>

        <!-- Main Content -->
        <div class="main-content">
            <div class="left-container">
                <!-- Loader files -->
                <div class="left-top-wrapper">
                    <div class="section-title">
                        <span>{{ t('config.title')}}</span>
                    </div>
                    <div class="form-group">
                        <label>{{ t('config.loader')}}</label>
                        <input type="text" class="file-input" id="loaderPathDisplay" value="res/devprg">
                        <button class="select-btn" id="btn_selectLoaderFile">{{ t('config.selectBtn')}}</button>
                    </div>
                    <div class="form-group">
                        <label>{{ t('config.digest')}}</label>
                        <input type="text" class="file-input" id="digestPathDisplay" value="res/digest">
                        <button class="select-btn" id="btn_selectDigestFile">{{ t('config.selectBtn')}}</button>
                    </div>
                    <div class="form-group">
                        <label>{{ t('config.sign')}}</label>
                        <input type="text" class="file-input" id="signPathDisplay" value="res/sig">
                        <button class="select-btn" id="btn_selectSignFile">{{ t('config.selectBtn')}}</button>
                    </div>
                    <div class="form-group">
                        <label>Raw XML:</label>
                        <input type="text" class="file-input" id="rawXmlPathDisplay" value="res/rawprogam0.xml">
                        <button class="select-btn" id="btn_selectRawXmlFile">{{ t('config.selectBtn')}}</button>
                    </div>
                    <div class="form-group">
                        <label>{{ t('config.edlFolder') }}:</label>
                        <input type="text" class="file-input" id="edlFolderPathDisplay" value="EDL Package Folder">
                        <button class="select-btn" id="btn_selectEdlFolder">{{ t('config.selectBtn')}}</button>
                    </div>
                </div>

                <div class="left-bottom-table-wrapper">
                    <div class="tab-nav">
                        <div v-for="tab in tabList"
                             :key="tab.key"
                             class="tab-item"
                             :class="{ active: activeTab === tab.key }"
                             @click="activeTab = tab.key">
                            {{ tab.label }}
                        </div>
                    </div>

                    <!-- Device Partition Table -->
                    <div class="part-table" v-show="activeTab === 'tab_part'">
                        <div class="table-header">
                            <input type="text" id="partFilter" :placeholder="$t('part.filter')">
                            <button id="selectAll" @click="selectAll">{{ t('part.selectAll') }}</button>
                        </div>
                        <div class="table-container">
                            <table>
                                <thead>
                                    <tr>
                                        <th v-for="col in tableColumns" :key="col.key" :style="{ width: col.width }">
                                            {{ col.label }}
                                        </th>
                                    </tr>
                                </thead>
                                <tbody class="part-table" id="partTable">
                                    <tr v-for="(item, index) in tableData" :key="index">
                                        <td><input v-model="item.chk" type='checkbox'></td>
                                        <td>{{ item.lun }}</td>
                                        <td class="partName">{{ item.partName }}</td>
                                        <td>{{ item.partSize }}</td>
                                        <td>{{ item.partStart }}</td>
                                        <td>{{ item.partNum }}</td>
                                        <td>{{ item.imgPath }}</td>
                                        <td><button id='{{ item.sel }}' @click="selectImgPath(item)">{{ t('config.selectBtn') }}</button></td>
                                    </tr>
                                </tbody>
                            </table>
                        </div>
                    </div>

                    <!-- EDL Package Panel -->
                    <div class="edl-panel" v-show="activeTab === 'tab_edl'">
                        <div class="edl-panel-left">
                            <v-stepper-vertical color="blue" v-model="activeStep" hide-actions>
                                    <v-stepper-vertical-item :complete="activeStep > 1" :subtitle="t('edl.step1')" title="Step 1" value="1">
                                        {{ t('edl.step1_content') }}
                                    </v-stepper-vertical-item>
                                    <v-stepper-vertical-item :complete="activeStep > 2" :subtitle="t('edl.step2')" title="Step 2" value="2">
                                        {{ t('edl.step2_content') }}
                                    </v-stepper-vertical-item>
                                    <v-stepper-vertical-item :complete="activeStep > 3" :subtitle="t('edl.step3')" title="Step 3" value="3">
                                        {{ t('edl.step3_content') }}
                                    </v-stepper-vertical-item>
                                    <v-stepper-vertical-item :complete="activeStep > 4" :subtitle="t('edl.step4')" title="Step 4" value="4">
                                        {{ t('edl.step4_content') }}
                                    </v-stepper-vertical-item>
                                    <v-stepper-vertical-item :complete="activeStep > 5" :subtitle="t('edl.step5')" title="Step 5" value="5">
                                        {{ t('edl.step5_content') }}
                                    </v-stepper-vertical-item>
                                    <v-stepper-vertical-item :complete="activeStep > 6" :subtitle="t('edl.step6')" title="Step 6" value="6">
                                        {{ t('edl.step6_content') }}
                                    </v-stepper-vertical-item>
                            </v-stepper-vertical>
                        </div>

                        <div class="edl-panel-right">
                            <div class="edl-panel-right-top">
                                <v-progress-circular :model-value="percentage" :rotate="360" :size="100" :width="15" color="#03fc5a">
                                    <template v-slot:default>
                                        {{ percentage }} %
                                    </template>
                                </v-progress-circular>
                            </div>
                            <div class="edl-panel-right-bottom">
                                <button class="edl-btn-green" v-show="edl_status == 0" @click="startFlashing()">{{ t('edl.start')}}</button>
                                <button class="edl-btn-red" v-show="edl_status == 1" @click="stopFlashing()">{{ t('edl.stop')}}</button>
                            </div>
                        </div>
                        
                        </div>
                    </div>
                </div>
            <div class="right-container">
                <!-- Reboot -->
                <div class="right-top-table-wrapper">
                    <div class="section-title">
                        <span>{{ t('reboot.title')}}</span>
                    </div>
                    <div class="btn-group">
                        <button class="btn-red" @click="rebootToSystem()">{{ t('reboot.system')}}</button>
                        <button class="btn-purple" @click="rebootToRecovery()">{{ t('reboot.recovery')}}</button>
                        <button class="btn-purple" @click="rebootToFastboot()">{{ t('reboot.fastboot')}}</button>
                        <button class="btn-red" @click="rebootToEdl()">{{ t('reboot.edl')}}</button>
                    </div>
                </div>

                <!-- Operation -->
                <div class="right-bottom-table-wrapper">
                    <form class="row" @submit.prevent="greet">
                        <div class="section-title">
                            <span>{{ t('operation.title') }}</span>
                        </div>
                        <div class="checkbox-group">
                            <label><input v-model="isBuildIn" type="checkbox">{{ t('operation.useBuildIn') }}</label>
                        </div>
                        <div class="radio-group">
                            <label><input type="radio" name="storage" checked> UFS</label>
                            <label><input type="radio" name="storage"> EMMC</label>
                        </div>
                        <div class="btn-group">
                            <button class="btn-blue" @click="sendLoader">{{ t('operation.sendLoader') }}</button>
                            <button class="btn-green" @click="readGPT">{{ t('operation.readGPT') }}</button>
                            <button class="btn-blue" @click="readPart">{{ t('operation.readPart') }}</button>
                            <button class="btn-orange" @click="writePart">{{ t('operation.writePart') }}</button>
                            <button class="btn-orange" @click="writeFromXML">{{ t('operation.writeFromXML') }}</button>
                            <button class="btn-brown" @click="saveToXML">{{ t('operation.createXML') }}</button>
                            <button class="btn-brown" @click="readDeviceInfo">{{ t('operation.readDeviceInfo') }}</button>
                            <button class="btn-red" @click="isDialogOpen = true">{{ t('operation.switchSlot') }}</button>
                        </div>
                    </form>
                </div>

                <!-- Log -->
                <div class="right-bottom-table-wrapper2">
                    <div class="section-title">
                        <span>{{ t('log.title') }}</span>
                        <button class="btn-red" @click="clearLog">{{ t('log.clearLog') }}</button>
                    </div>
                    <div class="log-section" id="logContainer">
                    </div>
                </div>
            </div>

        </div>
    </div>
</template>

<style scoped>
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}

</style>
<style>
    * {
        margin: 0;
        padding: 0;
        max-width: 100%;
        max-height: 100%;
        box-sizing: border-box;
    }
    html, body {
        max-width: 100vw;
        max-height: 100vh;
        background-color: #f0f2f5;
        padding: 2px;
    }
    .container {
        max-width: 100vw;
        max-height: 100vh;
        margin: 0 auto;
        background: linear-gradient(135deg, #5b86e5, #36d1dc);
        border-radius: 8px;
        box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        padding: 5px;
        color: white;
    }
    .header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 2px;
    }
    .header-left {
        display: flex;
        align-items: center;
        gap: 10px;
    }
    .header-left .status {
        color: #00ff9d;
        font-weight: bold;
    }
    .header-right {
        background-color: #1890ff;
        border: none;
        color: white;
        padding: 6px 12px;
        border-radius: 4px;
        cursor: pointer;
    }
    .main-content {
        display: flex;
        width: 95vw;
        height: 95vh;
        gap: 20px;
        min-height: 0;
    }
    /* Left container：60% width */
    .left-container {
        width: 60%;
        height: 100%;
        display: flex;
        flex-direction: column;
    }
    /* Right container: 40% width */
    .right-container {
        width: 40%;
        height: 100%;
        display: flex;
        flex-direction: column;
    }
    /* left top/bottom container */
    .left-top-table-wrapper {
        height: 40%;
    }
    .left-bottom-table-wrapper {
        height: 60%;
    }
    /* right top/bottom container */
    .right-top-table-wrapper {
        height: 15%;
    }
    .right-bottom-table-wrapper {
        height: 40%;
    }
    .right-bottom-table-wrapper2 {
        height: 45%;
    }
    .edl-panel {
        width: 100%;
        height: 100%;
        display: flex;
        gap: 20px;
        min-height: 0;
    }
    .edl-panel-left {
        width: 60%;
        height: 100%;
        display: flex;
    }
    .edl-panel-right {
        width: 40%;
        height: 100%;
        display: flex;
        flex-direction: column;
    }
    .edl-panel-right-top {
        width: 100%;
        height: 60%;
        display: flex;
        justify-content: center;
        align-items: center;
    }
    .edl-panel-right-bottom {
        width: 100%;
        height: 40%;
        display: flex;
        justify-content: center;
        align-items: center;
    }
    .table-container {
        max-height: 80%;
        overflow-y: auto;
    }
    .section {
        background-color: rgba(255,255,255,0.1);
        border-radius: 6px;
        padding: 10px;
    }
    .section-title {
        display: flex;
        align-items: center;
        gap: 4px;
        margin-bottom: 10px;
        font-size: 16px;
    }
    .form-group {
        display: flex;
        align-items: center;
        gap: 10px;
        margin-bottom: 10px;
    }
    .form-group label {
        width: 80px;
        font-size: 14px;
    }
    .form-group input {
        flex: 1;
        padding: 6px;
        border: none;
        border-radius: 4px;
        background-color: rgba(255,255,255,0.8);
    }
    .form-group button {
        background-color: #1890ff;
        border: none;
        color: white;
        padding: 4px 8px;
        border-radius: 4px;
        cursor: pointer;
    }
    .table-section {
        grid-column: 1 / 3;
    }
    .table-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 10px;
    }
    .table-header input {
        padding: 6px;
        border: none;
        border-radius: 4px;
        width: 90%;
        background-color: rgba(255,255,255,0.8);
    }
    .table-header button {
        background-color: #1890ff;
        border: none;
        color: white;
        padding: 6px 12px;
        border-radius: 4px;
        cursor: pointer;
    }
    table {
        width: 100%;
        border-collapse: collapse;
        background-color: rgba(255,255,255,0.9);
        color: #333;
        border-radius: 4px;
        overflow: hidden;
    }
    .part-table {
        height: 100%;
    }
    th, td {
        padding: 8px 12px;
        text-align: left;
        font-size: 14px;
        border-bottom: 1px solid #eee;
    }
    th {
        background-color: #f5f5f5;
    }
    .btn-group {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 8px;
        margin-bottom: 10px;
    }
    .btn-group button {
        padding: 2px;
        border: none;
        border-radius: 4px;
        color: white;
        cursor: pointer;
    }
    .btn-red {
        background-color: #ff4d4f;
    }
    .btn-purple {
        background-color: #722ed1;
    }
    .btn-blue {
        background-color: #1890ff;
    }
    .btn-green {
        background-color: #52c41a;
    }
    .btn-orange {
        background-color: #fa8c16;
    }
    .btn-brown {
        background-color: #8c6c4c;
    }
    .edl-btn-green {
        background-color: #52c41a;
        gap: 15px;
        border-radius: 5px; 
        padding: 20px;
        font-size: 18px;
    }
    .edl-btn-red {
        background-color: #ff4d4f;
        gap: 15px;
        border-radius: 5px;
        padding: 20px;
        font-size: 18px;
    }
    .radio-group {
        display: flex;
        gap: 15px;
        margin-bottom: 15px;
        font-size: 14px;
    }
    .checkbox-group {
        display: flex;
        gap: 15px;
        margin-bottom: 15px;
        font-size: 14px;
    }
    .log-section {
        background-color: rgba(0,0,0,0.5);
        border-radius: 4px;
        padding: 10px;
        height: 85%;
        overflow-y: auto;
        font-size: 12px;
        line-height: 1.5;
    }
    .open-dialog-btn {
        padding: 10px 20px;
        border: none;
        border-radius: 4px;
        background-color: #2563eb;
        color: white;
        cursor: pointer;
    }
    .open-dialog-btn:hover {
        background-color: #1d4ed8;
    }
    .slot-dialog {
        border: 1px solid #e2e8f0;
        border-radius: 6px;
        padding: 0px;
        width: 200px;
        text-align: center;
        margin: auto;
    }
    .dialog-title {
        margin: 0 0 20px 0;
        font-size: 16px;
        color: #1e293b;
    }
    .dialog-btn-group {
        display: flex;
        gap: 16px;
        justify-content: center;
    }
    .slot-btn {
        padding: 8px 24px;
        border: none;
        border-radius: 4px;
        color: white;
        cursor: pointer;
    }
    .slot-btn-a {
        background-color: #059669;
    }
    .slot-btn-a:hover {
        background-color: #047857;
    }
    .slot-btn-b {
        background-color: #d946ef;
    }
    .slot-btn-b:hover {
        background-color: #c026d3;
    }
    .tab-item.active {
        background-color: #d946ef;
        margin-bottom: -1px;
    }
    .tab-item:hover {
        background-color: #5b86e5;
    }
    .tab-nav {
        display: flex;
        background-color: inherit;
        border-bottom: 1px solid #ddd;
        border-radius: 4px;
        border: none;
    }
    .tab-item {
        padding: 5px 10px;
        cursor: pointer;
        border-right: 1px solid #ddd;
        transition: background 0.3s;
    }
    .tab-item.active {
        padding: 5px 10px;
        cursor: pointer;
        border-right: 1px solid #ddd;
        transition: background 0.3s;
    }
    .v-progress-circular {
        margin: 1rem;
    }
</style>
