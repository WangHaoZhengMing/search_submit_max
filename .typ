
// ========================= WARNINGS =========================

// ------------------------------------------------------------
// NOTE: The conversion did not work perfectly due to intrinsic
// Markdown to Typst limitations. The following custom
// functions, set or show rules are used to visually display
// these minor conversion warnings to the user.
// ------------------------------------------------------------

// EXTERNAL IMAGES WERE DETECTED!
#let external-image(url) = {
  rect(radius: 4pt, inset: 20pt,)[
    #align(center)[
      #text()[
        External image detected: \
        #link(url)
      ]
    ]
  ]
}

// ============================================================

① $D(2, 3)$；② $frac(S_(triangle.stroked.t C B D), S_(triangle.stroked.t A M N))$的最大值为 $frac(9, 16)$，此时 $D(frac(3, 2) , frac(15, 4))$

此答案由AI生成



解析

①过点D作$D H bot x$轴交x轴于H，连接$A C$、$B D$

#external-image(
  "https://k12static.xdf.cn/k12/xkw/1748718452721/86aa0f00-1112-4a97-904c-09f298e00665.png"
)   

则$angle D H B = angle C O A = 90 #none^circle.small$

∵$angle C A B = angle A B D$

∴$triangle.stroked.t C O A tilde.rev triangle.stroked.t D H B$

∴$frac(C O, D H) = frac(O A, B H)$

设$D(x, - x^2 + 2 x + 3)$

则$B H = 3 - x$，$D H = - x^2 + 2 x + 3$

∵$O C = 3$，$O A = 1$，

∴$frac(3, -x^2 + 2 x + 3) = frac(1, 3 - x)$

解得：$x_2 = 2$，$x_2 = 3$

∵当$x = 3$时，$-x^2 + 2 x + 3 = 0$

当$x = 2$时，$-x^2 + 2 x + 3 = 3$

∴$D(2, 3)$；

②连接$A M$、$A N$、$C D$、$O D$

#external-image(
  "https://k12static.xdf.cn/k12/xkw/1748718453071/636990e9-8434-4b99-a176-d9162378379a.png"
)   

设$D(m, - m^2 + 2 m + 3)$

∴$S_(triangle.stroked.t C B D) = S_(triangle.stroked.t C O D) + S_(triangle.stroked.t B O D) - S_(triangle.stroked.t B O C)$

$= frac(1, 2) times 3 times m + frac(1, 2) times 3 times(-m^2 + 2 m + 3) - frac(1, 2) times 3 times 3$

$= - frac(3, 2) m^2 + frac(9, 2) m$

∵$A(-1, 0)$，$B(3, 0)$，$A D "∥" B M$

∴设$y_(A D) = K_1 (x + 1)$，$y_(B D) = K_2 (x - 3)$，$y_(B M) = K_1 (x - 3)$

∵$K_1 = frac(-m^2 + 2 m + 3, m + 1) = -(m - 3)$

∴$y_(B M) = -(m - 3)(x - 3)$

$= -(m - 3) x + 3 m - 9$

∴$M(0, 3 m - 9)$

$k_2 = frac(-m^2 + 2 m + 3, 3 - m)$

$= -(m + 1)$

∴$y_(B D) = -(m + 1)(x - 3)$

$= -(m + 1) x + 3 m + 3$

∴$N(0, 3 m + 3)$

∴$M N = 3 m + 3 -(3 m - 9) = 12$

∴$S_(triangle.stroked.t A M N) = frac(1, 2) times 1 times 12 = 6$

∴$frac(S_(triangle.stroked.t C B D), S_(triangle.stroked.t A M N)) = frac(-frac(3, 2) m^2 + frac(9, 2) m, 6) = - frac(1, 4) m^2 + frac(3, 4) m$

∴当$m = frac(3, 2)$时，$frac(S_(triangle.stroked.t C B D), S_(triangle.stroked.t A M N)) = frac(9, 16)$最大

此时D(32,154)