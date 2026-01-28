// 1. 你的卷子列表
const paperList = [

                {"itemId": "3437257338438295552"},
                {"itemId": "3437256899915423744"},

];

// 2. 延时工具函数 (避免服务器封锁)
const sleep = (ms) => new Promise(resolve => setTimeout(resolve, ms));

// 3. 生成随机延时
const getRandomDelay = () => Math.floor(Math.random() * 200);

// 4. 执行函数
async function startBatchSubmit() {
  console.log(`🚀 开始批量提交，共 ${paperList.length} 份试卷...`);

  let successCount = 0;
  let failCount = 0;

  for (let i = 0; i < paperList.length; i++) {
    const item = paperList[i];
    const paperId = item.itemId; // 提取ID
    const currentNum = i + 1;
    
    console.log(`\n[${currentNum}/${paperList.length}] 正在提交 paperId: ${paperId}`);

    try {
      const response = await fetch("https://tps-tiku-api.staff.xdf.cn/paper/process/submit", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "Accept": "application/json, text/plain, */*",
          "tikutoken": "732FD8402F95087CD934374135C46EE5" // 确保这个Token没有过期
        },
        credentials: "include",
        body: JSON.stringify({
          "paperId": paperId, // 这里使用了 list 中的 itemId
          "type": "NEW_INPUT"
        })
      });

      const data = await response.json();

      // 简单的判断逻辑，你可以根据实际返回的 code 调整
      if (response.ok) {
         console.log(`✅ 提交成功 (ID: ${paperId})`, data);
         successCount++;
      } else {
         console.error(`❌ 服务器报错 (ID: ${paperId})`, data);
         failCount++;
      }

    } catch (err) {
      console.error(`❌ 请求网络错误 (ID: ${paperId})`, err);
      failCount++;
    }

    // 在每一份提交后，休息一下
    if (i < paperList.length - 1) {
       const waitTime = getRandomDelay();
       console.log(`⏳ 等待 ${waitTime}ms 后继续...`);
       await sleep(waitTime);
    }
  }

  console.log(`\n🎉 任务结束！成功: ${successCount}, 失败: ${failCount}`);
}

// 5. 启动
startBatchSubmit();
