<script setup>
    import { ref } from "vue";
    import { useI18n } from 'vue-i18n';
    import { invoke } from "@tauri-apps/api/core";
    import { locale as systemLocale } from "@tauri-apps/plugin-os";
    import { useAdvancedPanelEventHandler } from './composables/useAdvancedPanelEventHandler.js';
    import { useEdlPanelEventHandler } from './composables/useEdlPanelEventHandler.js';
    import { useEventListener } from './composables/useEventListener.js';
    import { useConfigPanelEventHandler } from './composables/useConfigPanelEventHandler.js';
    import { useOperationEventHandler } from './composables/useOperationEventHandler.js';
    import { useSettingPanelEventHandler } from './composables/useSettingPanelEventHandler.js';
    import { useStatusPanelEventHandler } from './composables/useStatusPanelEventHandler.js';
    import { useTableEventHandler } from './composables/useTableEventHandler.js';
    import { useRebootPanelEventHandler } from './composables/useRebootPanelEventHandler.js';

    const { t, locale, availableLocales } = useI18n();

    let activeTab = ref('tab_part');

    const tabList = ref([
        { key: 'tab_part', label: t('part.title') },
        { key: 'tab_edl', label: t('edl.title') },
        { key: 'tab_adv', label: t('advanced.title') },
        { key: 'tab_setting', label: t('setting.title') },
    ]);

    let {
        tableColumns,
        tableData,
        selectAll,
        selectImgPath,
        valueChangeListener,
    } = useTableEventHandler(t);

    let {
        portStatus,
        portName,
        selectedLang,
        handleSelectLangChange,
        updatePort,
    } = useStatusPanelEventHandler(locale, tableColumns, tabList, t);

    let {
        activeStep,
        slotDialogRef,
        isDialogOpen,
        isRunning,
        isCommandRunning,
        isSentLoader,
        percentage,
        working_percentage,
    } = useEventListener(tableData);

    let {
        imgSavingPath,
        isBuildIn,
        isProtectLun5,
        isEnablePing,
        isDebug,
        changeSavingPath,
        sendPing,
    } = useSettingPanelEventHandler(portName, isSentLoader, isCommandRunning);

    let {
        xmlContent,
        cmdOutput,
        selectedCmd,
        cmdList,
        runCommand,
        handleSelectCmdChange,
    } = useAdvancedPanelEventHandler(imgSavingPath, isDebug);

    let {
        rebootToEdl,
        rebootToFastboot,
        rebootToRecovery,
        rebootToSystem,
    } = useRebootPanelEventHandler(tableData, isDebug, t);

    let {
        erasePart,
        readDeviceInfo,
        readGPT,
        readPart,
        saveToXML,
        sendLoader,
        switchSlot,
        writeFromXML,
        writePart,
    } = useOperationEventHandler(imgSavingPath, isBuildIn, isDialogOpen, tableData, isDebug, t);

    async function clearLog() {
        logContainer.innerHTML = "";
    }

    let { startFlashing, stopFlashing } = useEdlPanelEventHandler(isRunning, isProtectLun5, isDebug);

    let {
        btn_selectLoaderFileClick,
        btn_selectDigestFileClick,
        btn_selectSignFileClick,
        btn_selectRawXmlFileClick,
        btn_selectEdlFolderClick,
    } = useConfigPanelEventHandler(tableData, activeTab, activeStep);

    window.onload = async function () {
        const systemlocale = await systemLocale();
        if (systemlocale) {
            selectedLang.value = systemlocale.replace('-', '_');
            handleSelectLangChange();
        }

        document.getElementById('partFilter').addEventListener('input', valueChangeListener);
    }
    
    setInterval(updatePort, 1000);
    setInterval(sendPing, 10000);
</script>

<template>
    <dialog ref="slotDialogRef"
            class="slot-dialog">
        <h3 class="dialog-title">{{ t('operation.switchSlot') }}</h3>
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
                <option value="ru">Russian (русский язык)</option>
                <option value="zh_CN">Simplified  Chinese (简体中文)</option>
                <option value="zh_TW">Traditional Chinese (正體中文)</option>
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
                        <button class="select-btn" id="btn_selectLoaderFile" @click="btn_selectLoaderFileClick">{{ t('config.selectBtn')}}</button>
                    </div>
                    <div class="form-group">
                        <label>{{ t('config.digest')}}</label>
                        <input type="text" class="file-input" id="digestPathDisplay" value="res/digest">
                        <button class="select-btn" id="btn_selectDigestFile" @click="btn_selectDigestFileClick">{{ t('config.selectBtn')}}</button>
                    </div>
                    <div class="form-group">
                        <label>{{ t('config.sign')}}</label>
                        <input type="text" class="file-input" id="signPathDisplay" value="res/sig">
                        <button class="select-btn" id="btn_selectSignFile" @click="btn_selectSignFileClick">{{ t('config.selectBtn')}}</button>
                    </div>
                    <div class="form-group">
                        <label>Raw XML:</label>
                        <input type="text" class="file-input" id="rawXmlPathDisplay" value="res/rawprogam0.xml">
                        <button class="select-btn" id="btn_selectRawXmlFile" @click="btn_selectRawXmlFileClick">{{ t('config.selectBtn')}}</button>
                    </div>
                    <div class="form-group">
                        <label>{{ t('config.edlFolder') }}:</label>
                        <input type="text" class="file-input" id="edlFolderPathDisplay" value="EDL Package Folder">
                        <button class="select-btn" id="btn_selectEdlFolder" @click="btn_selectEdlFolderClick">{{ t('config.selectBtn')}}</button>
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
                                <button class="edl-btn-green" v-show="isRunning == false" @click="startFlashing">{{ t('edl.start')}}</button>
                                <button class="edl-btn-red" v-show="isRunning == true" @click="stopFlashing">{{ t('edl.stop')}}</button>
                            </div>
                        </div>
                    </div>
                    <!-- Advanced Panel -->
                    <div class="adv-panel" v-show="activeTab === 'tab_adv'">
                        <div class="adv-panel-item">
                            <label>{{ t('advanced.selectCmd') }}</label>
                            <select class="header-right" name="template" id="template-select" v-model="selectedCmd" @change="handleSelectCmdChange">
                                <option v-for="item in cmdList" :key="item.id" :value="item.value">{{ item.label }}</option>
                            </select>
                        </div>
                        <div class="adv-panel-item">
                            <label class="img-folder-group-title">{{ t('setting.imgSavingPath') }}</label>
                            <textarea class="img-folder-group-path" v-model="imgSavingPath">img/</textarea>
                            <button class="img-folder-group-btn" @click="changeSavingPath">{{ t('setting.selectImgPathBtn') }}</button>
                        </div>
                        <div class="adv-panel-main">
                            <div class="adv-panel-main-left">
                                <label>{{ t('advanced.xmlContent') }}</label>
                                <textarea class="xml-content" id="xmlContent" v-model="xmlContent" cols="100" rows="5"></textarea>
                            </div>
                            <div class="adv-panel-main-right">
                                <label>{{ t('advanced.output') }}</label>
                                <textarea class="result-panel" v-model="cmdOutput" cols="100" rows="8" readonly></textarea>
                            </div>
                        </div>
                        <div class="adv-panel-item-center">
                            <button class="edl-btn-green" @click="runCommand">{{ t('advanced.run')}}</button>
                        </div>
                    </div>
                    <!-- Setting Panel -->
                    <div class="setting-panel" v-show="activeTab === 'tab_setting'">
                        <div class="img-folder-group">
                            <label class="img-folder-group-title">{{ t('setting.imgSavingPath') }}</label>
                            <textarea class="img-folder-group-path" v-model="imgSavingPath">img/</textarea>
                            <button class="img-folder-group-btn" @click="changeSavingPath">{{ t('setting.selectImgPathBtn') }}</button>
                        </div>
                        <div class="checkbox-group">
                            <label><input v-model="isBuildIn" type="checkbox">{{ t('setting.useBuildIn') }}</label>
                            <label><input v-model="isProtectLun5" type="checkbox" checked>{{ t('setting.protectLun5') }}</label>
                            <label><input v-model="isEnablePing" type="checkbox" checked>{{ t('setting.enablePing') }}</label>
                        </div>
                        <div class="radio-group">
                            <label>{{ t('setting.storageType') }}</label>
                            <label><input type="radio" name="storage" checked> UFS</label>
                            <label><input type="radio" name="storage"> EMMC</label>
                        </div>
                        <div class="radio-group">
                            <label>{{ t('setting.logLevel') }}</label>
                            <label><input type="radio" name="log" :value="false" v-model="isDebug" checked> Info</label>
                            <label><input type="radio" name="log" :value="true" v-model="isDebug"> Debug</label>
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
                        <button class="btn-red" @click="rebootToSystem">{{ t('reboot.system')}}</button>
                        <button class="btn-red" @click="rebootToEdl">{{ t('reboot.edl')}}</button>
                        <button class="btn-purple" @click="rebootToRecovery">{{ t('reboot.recovery')}}</button>
                        <button class="btn-purple" @click="rebootToFastboot">{{ t('reboot.fastboot')}}</button>
                    </div>
                </div>
                <!-- Operation -->
                <div class="right-bottom-table-wrapper">
                    <div class="row">
                        <div class="section-title">
                            <span>{{ t('operation.title') }}</span>
                        </div>
                        <div class="btn-group">
                            <button class="btn-blue" @click="sendLoader">{{ t('operation.sendLoader') }}</button>
                            <button class="btn-green" @click="readGPT">{{ t('operation.readGPT') }}</button>
                            <button class="btn-brown" @click="readDeviceInfo">{{ t('operation.readDeviceInfo') }}</button>
                            <button class="btn-brown" @click="saveToXML">{{ t('operation.createXML') }}</button>
                            <button class="btn-blue" @click="readPart">{{ t('operation.readPart') }}</button>
                            <button class="btn-orange" @click="writePart">{{ t('operation.writePart') }}</button>
                            <button class="btn-orange" @click="erasePart">{{ t('operation.erasePart') }}</button>
                            <button class="btn-orange" @click="writeFromXML">{{ t('operation.runCmdFromXML') }}</button>
                            <button class="btn-red" @click="isDialogOpen = true">{{ t('operation.switchSlot') }}</button>
                        </div>
                    </div>
                </div>
                <!-- Log -->
                <div class="right-bottom-table-wrapper2">
                    <div class="section-title">
                        <span>{{ t('log.title') }}</span>
                        <button class="btn-red" @click="clearLog">{{ t('log.clearLog') }}</button>
                    </div>
                    <div class="log-section" id="logContainer">
                    </div>
                    <v-progress-linear v-model="working_percentage" height="20" color="amber">
                        <strong>{{ working_percentage }}%</strong>
                    </v-progress-linear>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
    @import "style.css";
</style>
