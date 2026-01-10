import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { locale as systemLocale } from "@tauri-apps/plugin-os";

export function useStatusPanelEventHandler(locale, tableColumns, tabList, t) {

    let portStatus = ref("EDL device not found");
    let portName = ref("N/A");
    let portNum = ref("");
    let selectedLang = ref('en');

    const displayLang = {
        "en": "English",
        "ru": "Russian (русский язык)",
        "zh-CN": "Simplified  Chinese (简体中文)",
        "zh-TW": "Traditional Chinese (正體中文)",
    };


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
            { key: 'tab_adv', label: t('advanced.title') },
            { key: 'tab_setting', label: t('setting.title') },
        ];
    };

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

    onMounted(async () => {
        let systemlocale = await systemLocale();
        const match = systemlocale.match(/^[a-zA-Z]+/);
        const langCode = match ? match[0] : "";
        if (systemlocale in displayLang) {
            selectedLang.value = systemlocale;
        } else if (langCode in displayLang) {
            selectedLang.value = langCode;
        } else {
            selectedLang.value = 'en';
        }
        handleSelectLangChange();
    });

    return {
        portStatus,
        portName,
        selectedLang,
        displayLang,
        handleSelectLangChange,
        updatePort,
    }
}