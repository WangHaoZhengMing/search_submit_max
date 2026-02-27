// 1. 你的卷子列表 (原始数据)
// 假设这里是所有的 ID 对象或字符串
const sourceList = [

     
      
            {"taskId": 6386355,
                "itemId": "3428572140316405760",
            },
            {"taskId": 6386353,
                "itemId": "3428572123606298624",
            },
            {"taskId": 6386342,
                "itemId": "3428571732510818304",
            },
            {"taskId": 6386340,
                "itemId": "3428571395658117120",
            },
            {"taskId": 6386339,
                "itemId": "3428571375374221312",
            },
            {"taskId": 6386338,
                "itemId": "3428571360444297216",
            },
            {"taskId": 6386336,
                "itemId": "3428570899975028736",
            },
            {"taskId": 6386334,
                "itemId": "3428570853956923392",
            }


          
];

// 2. 延时工具函数
const sleep = (ms) => new Promise(resolve => setTimeout(resolve, ms));

// 3. 每次提交的 ID 数量
const BATCH_SIZE = 20; 

// 4. 单个批量请求函数
async function sendBatchRequest(idArray) {
    try {
        const response = await fetch("https://tps-tiku-api.staff.xdf.cn/task/paper/delete", {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
                "Accept": "application/json, text/plain, */*",
                "tikutoken": "732FD8402F95087CD934374135C46EE5" // ⚠️ 请确保 Token 有效
            },
            credentials: "include",
            // 核心修改：Body 直接就是一个字符串数组
            body: JSON.stringify(idArray) 
        });

        const data = await response.json();

        if (response.ok) {
            console.log(`✅ 本批次 ${idArray.length} 个提交成功`, data);
            return true;
        } else {
            console.error(`❌ 本批次提交失败:`, data);
            return false;
        }
    } catch (err) {
        console.error(`❌ 网络请求错误`, err);
        return false;
    }
}

// 5. 执行主函数
async function startBatchSubmit() {
    // 数据预处理：确保拿到的是纯 ID 数组
    // 如果你的 sourceList 里的元素是对象 (如 {itemId: "xxx"}), 需要 map 提取一下
    // 如果 sourceList 已经是 ["xxx", "xxx"] 格式，则直接使用
    const allIds = sourceList.map(item => item.itemId || item); 

    console.log(`🚀 开始处理，共 ${allIds.length} 个 ID，每次打包 ${BATCH_SIZE} 个提交...`);
    
    let successBatches = 0;

    // 分批循环
    for (let i = 0; i < allIds.length; i += BATCH_SIZE) {
        // 截取 10 个 ID 组成一个数组
        const idBatch = allIds.slice(i, i + BATCH_SIZE);
        const currentBatchNum = Math.floor(i / BATCH_SIZE) + 1;
        
        console.log(`\n--- 正在提交第 ${currentBatchNum} 批 (包含 ${idBatch.length} 个ID) ---`);
        
        // 打印一下即将发送的 Payload 格式以供检查
        // console.log("Payload:", JSON.stringify(idBatch));

        const isSuccess = await sendBatchRequest(idBatch);
        if (isSuccess) successBatches++;

        // 批次之间等待，防止请求过快
        if (i + BATCH_SIZE < allIds.length) {
            const waitTime = Math.floor(Math.random() * 500) ; // 0-500ms 随机等待
            console.log(`⏳ 等待 ${waitTime}ms 后发送下一批...`);
            await sleep(waitTime);
        }
    }

    console.log(`\n🎉 任务结束！成功发送批次: ${successBatches}`);
}

// 6. 启动
startBatchSubmit();