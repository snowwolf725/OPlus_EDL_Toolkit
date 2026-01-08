import { ref, watch } from "vue";
import { listen } from '@tauri-apps/api/event';
import { XMLParser } from 'fast-xml-parser';

export function useEventListener(tableData) {

    let activeStep = ref(1);
    let slotDialogRef = ref(null);
    let isDialogOpen = ref(false);
    let isRunning = ref(false);
    let isCommandRunning = false;
    let isSentLoader = ref(false);
    let percentage = ref(0);
    let working_percentage = ref(0);

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

    listen("stop_edl_flashing", (payload) => {
        isRunning.value = false;
    });

    listen("update_command_running_status", (payload) => {
        isCommandRunning = payload.payload;
    });

    listen("update_loader_status", (payload) => {
        isSentLoader.value = payload.payload;
    });

    listen("update_percentage", (payload) => {
        percentage.value = payload.payload;
        if (percentage.value >= 100) {
            activeStep.value = 7;
        } else if (percentage.value >= 95) {
            activeStep.value = 6;
        } else if (percentage.value >= 80) {
            activeStep.value = 5;
        } else if (percentage.value >= 20) {
            activeStep.value = 4;
        } else if (percentage.value >= 10) {
            activeStep.value = 3;
        } else if (percentage.value >= 5) {
            activeStep.value = 2;
        } else {
            activeStep.value = 1;
        }
    });

    listen("update_working_percentage", (payload) => {
        working_percentage.value = payload.payload;
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

    return {
        activeStep,
        slotDialogRef,
        isDialogOpen,
        isRunning,
        isCommandRunning,
        isSentLoader,
        percentage,
        working_percentage,
    }
}
