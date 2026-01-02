import { ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";

export function useTableEventHandler(t) {

    const tableColumns = ref([
        { key: 'chk', label: '', width: '5%' },
        { key: 'lun', label: 'LUN', width: '5%' },
        { key: 'partName', label: t('part.name'), width: '10%' },
        { key: 'partSize', label: t('part.size'), width: '10%' },
        { key: 'partStart', label: t('part.start'), width: '10%' },
        { key: 'partNum', label: t('part.num'), width: '10%' },
        { key: 'imgPath', label: t('part.imgPath'), width: '40%' },
        { key: 'sel', label: t('config.selectBtn'), width: '10%' },
    ]);

    const tableData = ref([]);

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

    const selectImgPath = async (item) => {
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

    async function valueChangeListener() {
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
    }

    return {
        tableColumns,
        tableData,
        selectAll,
        selectImgPath,
        valueChangeListener,
    }
}