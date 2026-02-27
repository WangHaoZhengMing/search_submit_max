#import "@preview/lilaq:0.5.0" as lq
#set page(width: auto,height: auto)
// 设置字体，使其符合中文排版的标准学术风格
#set text(font: ("Times New Roman", "SimSun"), size: 10pt)

// 1. 定义从图片中提取的数据
// 气温数据（1到12月，单位：℃）
#let temperature = (2, 3, 5, 8, 12, 15, 17, 16, 12, 9, 5, 3)
// 降水量数据（1到12月，单位：mm）
#let precipitation = (90, 75, 50, 40, 30, 55, 65, 60, 50, 65, 55, 55)

#figure(
  align(center)[
    // 留出边距给顶部的文字标签
    #pad()[
      #lq.diagram(
        width: 5.5cm,
        height: 7cm,
        
        // ---------- X轴配置 ----------
        xaxis: (
          lim: (0.5, 12.5),  // 保证柱子不会紧贴左右边缘
          mirror: false,     // 隐藏顶部的边框线
          subticks: none,
          ticks: (
            (1, "1"), (2, "2"), (3, "3"), (4, "4"), (5, "5"), 
            (6, "6"), (7, "7"), (8, "8"), (9, "9"), 
            (10, [10]),      // 只有第10个月有数字标注
            (11, "11"), (12, "12")
          ),
        ),
        
        // ---------- 左侧 Y轴配置 (气温) ----------
        yaxis: (
          lim: (-60, 30),
          mirror: false,
          ticks: (-60, -45, -30, -15, 0, 15, 30),
        ),
        
        // 绘制贯穿的水平背景网格线 (由于图片网格线是实线，我们手动绘制更精确)
        ..(-60, -45, -30, -15, 0, 15, 30).map(y => 
          lq.plot((0.5, 12.5), (y, y), stroke: black + 0.5pt)
        ),
        
        // ---------- 右侧 Y轴配置 (降水量) ----------
        lq.yaxis(
          position: right,
          lim: (0, 600),
          mirror: false,
          ticks: (0, 100, 200, 300, 400, 500, 600),
          
          // 降水量柱状图 (绑定在右侧Y轴上)
          lq.bar(
            range(1, 13),
            precipitation,
            fill: black,
            width: 0.8
          )
        ),
        
        // ---------- 气温折线图 ----------
        lq.plot(
          range(1, 13),
          temperature,
          color: black,
          stroke: 1pt,
          mark: "o",       // 实心圆点
          mark-size: 3pt,
        ),
        
        // ---------- 手动放置角落的文字标签 (已修复报错) ----------
        // 气温/℃：利用 move 将文本整体上移 1.5em
        lq.place(0.5, 30, align: center, move(dy: -1.5em)[气温/℃]),
        
        // 降水量/mm：利用 move 将文本整体上移 1.5em
        lq.place(12.5, 30, align: center, move(dy: -1.5em)[降水量/mm]),
        
        // (月)：放置在右下角，利用 move 稍微向右下方避开坐标轴数字
        lq.place(12.5, -60, align: right, move(dx: 1.5em, dy: 1em)[(月)]),
      )
    ]
  ]
)