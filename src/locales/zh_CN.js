export default {
	config: {
		title: '引导和刷机文件',
		loader: '引导文件:',
		digest: 'Digest文件:',
		sign: 'Sign:',
		selectBtn: '选择',
		portStatus: 'EDL设备已连接',
		portStatusError: '未找到EDL设备',
	},
	part: {
		title: '设备分区表',
		filter: '使用分区名称筛选',
		selectAll: '全选',
		name: '分区名称',
		size: '大小',
		start: '起始扇区',
		num: '扇区数',
		imgPath: '镜像文件',
	},
	reboot: {
		title: '重启至',
		system: '重启到系统',
		recovery: '重启到Recovery',
		fastboot: '重启到fastboot',
		edl: '重启到EDL模式',
		miscNotFound: "在分区列表中无法找到misc分区",
	},
	operation: {
		title: '功能操作',
		sendLoader: '发送引导',
		readGPT: '读取分区表',
		readPart: '提取选中分区',
		writePart: '写入选中分区',
		writeFromXML: 'XML写入固件',
		createXML: '保存至XML',
		saveAlert: '请至少选取一个分区',
	},
	log: {
		title: '操作日志',
	}
}