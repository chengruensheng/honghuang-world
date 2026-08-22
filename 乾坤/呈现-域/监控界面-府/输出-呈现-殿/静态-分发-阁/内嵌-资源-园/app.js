// 监控界面前端 —— 分裂流（Linear Aesthetic v=4）
// 保留：装配步骤流 / SSE / 分裂合流检测 / 视图切换 / 任务列表分组
// 改进：SVG 内联图标 + 域着色 + 微光动画 + 玻璃拟态
(function () {
  "use strict";
  var 流 = null;
  var MAX = 200;
  var state = {
    事件池: [],
    视图: "步骤流",
    选中任务线: null,
    活跃线: new Set(),
    拓朴: [],
    步骤流: {},
    回放游标: null,
    旁路产出: [],
    分裂点: [],
    装配步骤: [],
    并行块: null,
    矛盾清单: null,
    统计: { 事件数: 0, token: 0, 耗时ms: 0 }
  };

  function $(id) { return document.getElementById(id); }

  function 格式时刻(ts) {
    if (!ts) return "";
    var d = new Date(ts);
    var h = String(d.getHours()).padStart(2, "0");
    var m = String(d.getMinutes()).padStart(2, "0");
    var s = String(d.getSeconds()).padStart(2, "0");
    var ms = String(d.getMilliseconds()).padStart(3, "0");
    return h + ":" + m + ":" + s + "." + ms;
  }

  // SVG 图标构造（引用 index.html 内联 <symbol> 定义）
  function 图标(id, 类名) {
    var svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    if (类名) svg.setAttribute("class", 类名);
    var use = document.createElementNS("http://www.w3.org/2000/svg", "use");
    use.setAttribute("href", "#" + id);
    svg.appendChild(use);
    return svg;
  }

  // 取动作对应的状态图标 id
  function 状态图标id(步, 活跃) {
    if (活跃) return "ic-spin";
    if (步.完成) return "ic-done";
    var 域 = 步.域 || "";
    if (域.indexOf("提示词") >= 0) return "ic-prompt";
    if (域.indexOf("思考") >= 0) return "ic-think";
    if (域.indexOf("回复") >= 0) return "ic-reply";
    if (域.indexOf("工具调用") >= 0 || 域.indexOf("工具") >= 0) return "ic-tool";
    if (域.indexOf("返回") >= 0) return "ic-return";
    return "ic-think";
  }

  // 取组件图标 id
  function 组件图标id(域) {
    if (!域) return "ic-tool";
    if (域.indexOf("工具调用") >= 0 || 域.indexOf("工具") >= 0) return "ic-tool";
    if (域.indexOf("返回") >= 0) return "ic-return";
    if (域.indexOf("提示词") >= 0) return "ic-prompt";
    if (域.indexOf("思考") >= 0) return "ic-think";
    if (域.indexOf("回复") >= 0) return "ic-reply";
    return "ic-tool";
  }

  // 八态着色映射
  function 态类(动作) {
    if (!动作) return "";
    if (动作.indexOf("思考") >= 0) return "态-思考";
    if (动作.indexOf("工具") >= 0 || 动作.indexOf("读文件") >= 0 || 动作.indexOf("写文件") >= 0 || 动作.indexOf("跑命令") >= 0) return "态-工具";
    if (动作.indexOf("等待") >= 0) return "态-等待";
    if (动作.indexOf("完成") >= 0 || 动作.indexOf("✓") >= 0) return "态-完成";
    if (动作.indexOf("失败") >= 0 || 动作.indexOf("⚠") >= 0) return "态-失败";
    if (动作.indexOf("验收") >= 0 || 动作.indexOf("审验") >= 0) return "态-验收";
    if (动作.indexOf("设计") >= 0) return "态-设计";
    if (动作.indexOf("派遣") >= 0 || 动作.indexOf("分裂") >= 0) return "态-派遣";
    return "";
  }

  // 按域着色（提示词/思考/回复/工具/返回）
  function 域类(域, 动作) {
    var s = (域 || "") + " " + (动作 || "");
    if (!s.trim()) return "";
    if (s.indexOf("提示词") >= 0) return "态-提示词";
    if (s.indexOf("思考") >= 0) return "态-思考";
    if (s.indexOf("回复") >= 0) return "态-回复";
    if (s.indexOf("工具调用") >= 0 || s.indexOf("工具") >= 0) return "态-工具";
    if (s.indexOf("返回") >= 0) return "态-返回";
    return "";
  }

  // LLM 事件判定
  function 是LLM事件(e) {
    var 动作 = e.动作 || "";
    var 域 = e.域 || "";
    return 动作.indexOf("提示词") >= 0 || 动作.indexOf("思考") >= 0 || 动作.indexOf("回复") >= 0
      || 域.indexOf("提示词") >= 0 || 域.indexOf("回复") >= 0;
  }

  // 工具事件判定
  function 是工具事件(e) {
    var 动作 = e.动作 || "";
    var 域 = e.域 || "";
    return 动作.indexOf("工具") >= 0 || 域.indexOf("工具") >= 0;
  }

  function 取步骤标题(e) {
    var 标题 = e.动作 || e.域 || "模型思考";
    return String(标题).slice(0, 60);
  }

  function 合并token(token) {
    if (!token) return { 总计: 0, 提示词: 0, 输出: 0, 缓存: 0 };
    return {
      总计: token.总计 || 0,
      提示词: token.提示词 || 0,
      输出: token.输出 || 0,
      缓存: token.缓存 || 0
    };
  }

  function 截断证据(文本, 长度) {
    var s = String(文本 || "");
    if (s.length > 长度) return s.slice(0, 长度) + "…";
    return s;
  }

  // 去标记：删 think 标签、output 闭合标签、截超长 JSON 段、合并空白
  function 去标记(文本) {
    var s = String(文本 || "");
    s = s.replace(/<antThinking>/gi, "").replace(/<\/antThinking>/gi, "");
    s = s.replace(/<\/output>/gi, "");
    // 反复截断超长 JSON 大括号段（处理嵌套，最多 3 轮）
    for (var i = 0; i < 3; i++) {
      var prev = s;
      s = s.replace(/\{[^{}]*\}/g, function (m) {
        return m.length > 50 ? "[JSON]" : m;
      });
      if (prev === s) break;
    }
    s = s.replace(/\s+/g, " ").trim();
    return s;
  }

  // 提炼证据摘要：按域分派提取一行可读摘要
  function 提炼证据摘要(文本, 域) {
    var s = String(文本 || "");
    if (!s) return "";
    var d = String(域 || "");

    function 取路径() {
      var m = s.match(/(?:[A-Za-z]:[\/\\])?[^\s"'<>|]*[\/\\][^\s"'<>|]+\.[a-zA-Z]{1,8}/);
      if (m) return m[0];
      m = s.match(/[^\s"'<>|]+\.(rs|toml|md|js|css|html|json|py|go|java|ts|tsx|jsx|txt|log|yaml|yml|sh|bat|ps1|xml|csv)/i);
      return m ? m[0] : "";
    }
    function 取行数() {
      var m = s.match(/(\d+)\s*行/) || s.match(/lines?\s*[:\s]*(\d+)/i) || s.match(/(\d+)\s*lines?/i);
      return m ? m[1] : "";
    }
    function 取退出码() {
      var m = s.match(/退出码?\s*[:\s]*(\d+)/) || s.match(/exit\s*code\s*[:\s]*(\d+)/i) || s.match(/exit\s*[:\s]*(\d+)/i) || s.match(/退出\s*(\d+)/);
      return m ? m[1] : "";
    }

    // 读文件：路径 + 行数
    if (d.indexOf("读文件") >= 0 || (d.indexOf("工具") >= 0 && /读|read/i.test(s))) {
      var p = 取路径();
      var n = 取行数();
      if (p) return "读取 " + p + (n ? " → " + n + "行" : "");
    }
    // 写文件：路径 + 状态
    if (d.indexOf("写文件") >= 0 || (d.indexOf("工具") >= 0 && /写|write/i.test(s))) {
      var p = 取路径();
      if (p) {
        var 状态 = /完成|成功|✓|ok/i.test(s) ? "完成" : (/失败|错误|⚠|err/i.test(s) ? "失败" : "");
        return "写入 " + p + (状态 ? " → " + 状态 : "");
      }
    }
    // 跑命令：命令名 + 退出码
    if (d.indexOf("跑命令") >= 0 || (d.indexOf("工具") >= 0 && /命令|cargo|npm|git|python|node|rustc|go\s|make|cmake|pip/i.test(s))) {
      var m = s.match(/(cargo|npm|git|python|node|rustc|go|make|cmake|pip|pnpm|yarn|tsc|eslint|prettier|clippy|fmt)[\w\-]*/i);
      var cmd = m ? m[0] : "";
      var code = 取退出码();
      if (cmd) return cmd + (code ? " → 退出" + code : "");
    }
    // 提示词：去标记后前 40 字
    if (d.indexOf("提示词") >= 0) {
      return 截断证据(去标记(s), 40);
    }
    // 思考：提取 <antThinking> 后首句
    if (d.indexOf("思考") >= 0) {
      var think = s.match(/<antThinking>([\s\S]*?)<\/antThinking>/i);
      var 内容 = think ? think[1] : s;
      var clean = 去标记(内容);
      var 首句 = clean.split(/[。；;\n]/)[0];
      return 截断证据(首句, 60);
    }
    // 回复：去标记后前 60 字
    if (d.indexOf("回复") >= 0) {
      return 截断证据(去标记(s), 60);
    }
    // 返回：状态 + 行数
    if (d.indexOf("返回") >= 0) {
      var 状态 = /成功|完成|✓|ok/i.test(s) ? "成功" : (/失败|错误|⚠|err/i.test(s) ? "失败" : "");
      var n = 取行数();
      if (状态) return 状态 + (n ? " · " + n + "行" : "");
    }
    // 其他：去标记后前 50 字
    return 截断证据(去标记(s), 50);
  }

  function 取线id(e) {
    return e["任务线id"] || e.任务线id || (e.线 && e.线.id) || null;
  }

  // token 进度条填充百分比（对数缩放，避免大值压扁小值）
  function token填充率(值) {
    if (!值 || 值 <= 0) return 0;
    var p = Math.log10(值 + 1) / Math.log10(10000);
    if (p > 1) p = 1;
    return Math.round(p * 100);
  }

  // 耗时人类可读
  function 耗时文本(ms) {
    if (!ms || ms <= 0) return "0ms";
    if (ms < 1000) return ms + "ms";
    if (ms < 60000) return (ms / 1000).toFixed(1) + "s";
    return Math.floor(ms / 60000) + "m" + Math.floor((ms % 60000) / 1000) + "s";
  }

  // ===== 本地装配步骤流（保留原逻辑） =====
  function 装配步骤流(事件们) {
    var 步骤们 = [];
    var 当前 = null;
    事件们.forEach(function (e) {
      if (是LLM事件(e)) {
        if (当前) {
          当前.完成 = true;
          步骤们.push(当前);
        }
        当前 = {
          步骤号: 步骤们.length + 1,
          ts: e.ts,
          标题: 取步骤标题(e),
          域: e.域 || "",
          任务线id: 取线id(e),
          token: 合并token(e.token),
          耗时ms: e["耗时ms"] || 0,
          完成: false,
          组件: [],
          证据: e.证据 || ""
        };
      } else if (是工具事件(e)) {
        // 跳过空工具事件（无证据+token=0+耗时=0），避免组件项爆炸
        var 工具证据 = e.证据 || "";
        var 工具token = (e.token && e.token.总计) || 0;
        var 工具耗时 = e["耗时ms"] || 0;
        if (工具证据.length <= 2 && 工具token === 0 && 工具耗时 === 0) return;
        if (!当前) {
          当前 = {
            步骤号: 步骤们.length + 1,
            ts: e.ts,
            标题: "工具调用",
            域: e.域 || "工具调用",
            任务线id: 取线id(e),
            token: { 总计: 0, 提示词: 0, 输出: 0, 缓存: 0 },
            耗时ms: 0,
            完成: false,
            组件: [],
            证据: ""
          };
        }
        当前.组件.push({
          名: e.动作 || "工具",
          域: e.域 || "工具调用",
          证据: e.证据 || "",
          token: 合并token(e.token),
          耗时ms: e["耗时ms"] || 0
        });
        当前.token.总计 += (e.token && e.token.总计) || 0;
        当前.token.提示词 += (e.token && e.token.提示词) || 0;
        当前.token.输出 += (e.token && e.token.输出) || 0;
        当前.token.缓存 += (e.token && e.token.缓存) || 0;
        当前.耗时ms += e["耗时ms"] || 0;
      } else {
        // 跳过空其他事件
        var 其他证据 = e.证据 || "";
        var 其他token = (e.token && e.token.总计) || 0;
        var 其他耗时 = e["耗时ms"] || 0;
        if (其他证据.length <= 2 && 其他token === 0 && 其他耗时 === 0) return;
        if (当前) {
          当前.组件.push({
            名: e.动作 || "其他",
            域: e.域 || "",
            证据: e.证据 || "",
            token: 合并token(e.token),
            耗时ms: e["耗时ms"] || 0
          });
          当前.token.总计 += (e.token && e.token.总计) || 0;
          当前.耗时ms += e["耗时ms"] || 0;
        }
      }
    });
    if (当前) 步骤们.push(当前);
    return 步骤们;
  }

  // ===== 渲染步骤卡片（卡牌式三段：主题 / 主体 / 结论） =====
  // 主题：状态图标 + 步骤号 + 时间戳 + 线标签（行1），动作标题完整显示可换行（行2）
  // 主体：证据全文 + 组件项，固定 max-height 内部滚动，始终显示
  // 结论：结果摘要 + token + 耗时 + 箭头，点击切换主体展开/收起
  function 渲染步骤卡片(步, 活跃, 默认展开, 显示线标签) {
    var div = document.createElement("div");
    div.className = "步骤卡片 " + 域类(步.域, 步.标题) + (活跃 ? " 活跃" : (步.完成 ? " 完成" : ""));
    if (默认展开) div.classList.add("展开");

    var 动作摘要 = 取步骤标题(步);
    var 结果摘要 = 提炼证据摘要(步.证据, 步.域);

    // === 主题（顶部标题栏）===
    var 主题 = document.createElement("div");
    主题.className = "卡片主题";

    // 主题行1：状态图标 + 步骤号 + 时间戳 + 线标签
    var 主题行1 = document.createElement("div");
    主题行1.className = "主题行1";

    var 状态图标盒 = document.createElement("span");
    状态图标盒.className = "状态图标";
    状态图标盒.appendChild(图标(状态图标id(步, 活跃)));
    主题行1.appendChild(状态图标盒);

    var 号 = document.createElement("span");
    号.className = "号";
    号.textContent = "步骤 " + (步.步骤号 || "·");
    主题行1.appendChild(号);

    // 卡片毫秒时间戳（瀑布式展示：每块带时刻）
    var 时刻 = document.createElement("span");
    时刻.className = "卡片时刻";
    时刻.textContent = 格式时刻(步.ts);
    主题行1.appendChild(时刻);

    // 任务线标签（仅在线id变化时显示，由调用方决定）
    if (显示线标签 && 步.任务线id) {
      var 线标签 = document.createElement("span");
      线标签.className = "线标签 " + 域类(步.域, 步.标题);
      线标签.textContent = "线" + 步.任务线id;
      主题行1.appendChild(线标签);
    }

    主题.appendChild(主题行1);

    // 主题行2：动作标题（完整显示不截断，允许换行）
    var 主题行2 = document.createElement("div");
    主题行2.className = "主题行2";
    主题行2.textContent = 结果摘要 ? (动作摘要 + " → " + 结果摘要) : 动作摘要;
    主题.appendChild(主题行2);

    div.appendChild(主题);

    // === 主体（中部内容区，固定高度内部滚动，始终显示）===
    var 主体 = document.createElement("div");
    主体.className = "卡片主体";

    // 主证据：完整显示原始证据全文（pre-wrap，固定高度内部滚动）
    if (步.证据) {
      var 主证据 = document.createElement("div");
      主证据.className = "主证据";
      主证据.textContent = 步.证据;
      主体.appendChild(主证据);
    }

    // 组件项：每个组件项也是三段式小卡牌（组件名=主题，组件证据=主体，token/耗时=结论）
    if (步.组件 && 步.组件.length) {
      步.组件.forEach(function (c) {
        var 项 = document.createElement("div");
        项.className = "组件项 " + 域类(c.域, c.名);

        // 组件主题：组件图标 + 组件名
        var 组件主题 = document.createElement("div");
        组件主题.className = "组件主题";

        var 组件图标盒 = document.createElement("span");
        组件图标盒.className = "组件图标";
        组件图标盒.appendChild(图标(组件图标id(c.域)));
        组件主题.appendChild(组件图标盒);

        var 组件名 = document.createElement("span");
        组件名.className = "组件名";
        组件名.textContent = String(c.名 || "·");
        组件主题.appendChild(组件名);

        项.appendChild(组件主题);

        // 组件主体：组件证据全文（pre-wrap，可换行不截断）
        if (c.证据) {
          var 组件证据 = document.createElement("div");
          组件证据.className = "组件证据";
          组件证据.textContent = c.证据;
          项.appendChild(组件证据);
        }

        // 组件结论：组件 token + 组件耗时
        var 组件结论 = document.createElement("div");
        组件结论.className = "组件结论";

        var cToken = (c.token && c.token.总计) || 0;
        if (cToken > 0) {
          var 组件token = document.createElement("span");
          组件token.className = "组件token";
          组件token.textContent = "tk " + cToken;
          组件结论.appendChild(组件token);
        }

        var c耗时 = c["耗时ms"] || 0;
        if (c耗时 > 0) {
          var 组件耗时 = document.createElement("span");
          组件耗时.className = "组件耗时";
          组件耗时.textContent = 耗时文本(c耗时);
          组件结论.appendChild(组件耗时);
        }

        if (组件结论.childNodes.length) 项.appendChild(组件结论);

        主体.appendChild(项);
      });
    }

    div.appendChild(主体);

    // === 结论（底部结果栏）===
    var 结论 = document.createElement("div");
    结论.className = "卡片结论";

    // 结果摘要（若有）
    if (结果摘要) {
      var 摘要盒 = document.createElement("span");
      摘要盒.className = "结果摘要";
      摘要盒.textContent = 结果摘要;
      结论.appendChild(摘要盒);
    }

    // token 进度条 + 数值（仅 token.总计 > 0 时渲染）
    var token总计 = (步.token && 步.token.总计) || 0;
    if (token总计 > 0) {
      var token条 = document.createElement("span");
      token条.className = "token条";
      var token进度 = document.createElement("span");
      token进度.className = "token进度";
      var 填充 = document.createElement("span");
      填充.className = "填充";
      填充.style.width = token填充率(token总计) + "%";
      token进度.appendChild(填充);
      token条.appendChild(token进度);
      var token数值 = document.createElement("span");
      token数值.className = "token数值";
      token数值.textContent = "tk " + token总计;
      token条.appendChild(token数值);
      结论.appendChild(token条);
    }

    // 耗时（仅 耗时ms > 0 时渲染）
    var 耗时ms = 步["耗时ms"] || 0;
    if (耗时ms > 0) {
      var 耗时 = document.createElement("span");
      耗时.className = "耗时";
      耗时.textContent = 耗时文本(耗时ms);
      结论.appendChild(耗时);
    }

    // 展开/收起箭头
    var 箭头盒 = document.createElement("span");
    箭头盒.className = "箭头";
    箭头盒.appendChild(图标("ic-chevron"));
    结论.appendChild(箭头盒);

    div.appendChild(结论);

    // 点击结论栏切换主体展开/收起（默认 200px，展开 600px）
    结论.addEventListener("click", function () {
      div.classList.toggle("展开");
    });
    return div;
  }

  // ===== 全量渲染步骤流（保留原结构，分裂点用 SVG 图标） =====
  function 渲染步骤流全量() {
    var 流div = $("事件流");
    流div.innerHTML = "";
    var 步骤们 = 装配步骤流(state.事件池);
    state.装配步骤 = 步骤们;

    var 混合 = [];
    步骤们.forEach(function (步) { 混合.push({ 类型: "步骤", ts: 步.ts || 0, 数据: 步 }); });
    state.分裂点.forEach(function (点) { 混合.push({ 类型: 点.类型, ts: 点.ts, 数据: 点 }); });
    混合.sort(function (a, b) { return a.ts - b.ts; });

    var 最后未完成 = -1;
    for (var i = 步骤们.length - 1; i >= 0; i--) {
      if (!步骤们[i].完成) { 最后未完成 = i; break; }
    }

    var 上一步线id = null;
    混合.forEach(function (项) {
      if (项.类型 === "步骤") {
        var 步骤索引 = 步骤们.indexOf(项.数据);
        var 活跃 = 步骤索引 === 最后未完成;
        var 默认展开 = 步骤索引 === 步骤们.length - 1;
        var 当前线id = 项.数据.任务线id || null;
        var 显示线标签 = 当前线id !== null && 当前线id !== 上一步线id;
        流div.appendChild(渲染步骤卡片(项.数据, 活跃, 默认展开, 显示线标签));
        上一步线id = 当前线id;
      } else {
        流div.appendChild(渲染分裂合流点(项.类型, 项.ts));
        if (项.类型 === "合流" && state.矛盾清单 && state.矛盾清单.length) {
          流div.appendChild(渲染汇合面板(state.矛盾清单));
        }
      }
    });

    if (state.并行块 && state.并行块.length) {
      流div.appendChild(渲染并行块(state.并行块));
    }

    // 最新块高亮（瀑布式展示：始终突出最新块）
    var 卡片们 = 流div.querySelectorAll(".步骤.卡片");
    if (卡片们.length > 0) {
      var 最新 = 卡片们[卡片们.length - 1];
      最新.classList.add("最新块");
      setTimeout(function () { 最新.classList.remove("最新块"); }, 2000);
    }

    流div.scrollTop = 流div.scrollHeight;
  }

  // 分裂/合流点（SVG 图标 + 时间戳）
  function 渲染分裂合流点(类型, ts) {
    var div = document.createElement("div");
    div.className = 类型 === "分裂" ? "分裂点" : "合流点";

    var 图标盒 = document.createElement("span");
    图标盒.className = "点图标";
    图标盒.appendChild(图标(类型 === "分裂" ? "ic-fork" : "ic-merge"));
    div.appendChild(图标盒);

    var 标签 = document.createElement("span");
    标签.className = "点标签";
    标签.textContent = 类型 === "分裂" ? "分裂" : "合流";
    div.appendChild(标签);

    var 时间 = document.createElement("span");
    时间.className = "点时间";
    时间.textContent = 格式时刻(ts);
    div.appendChild(时间);

    return div;
  }

  // 渲染单步（任务树视图用，简化形态：主题 + 结论，无主体）
  function 渲染步骤(步, 活跃) {
    var div = document.createElement("div");
    div.className = "步骤卡片 " + 域类(步.域, 步.标题) + (活跃 ? " 活跃" : (步.完成 ? " 完成" : ""));

    // 主题
    var 主题 = document.createElement("div");
    主题.className = "卡片主题";

    var 主题行1 = document.createElement("div");
    主题行1.className = "主题行1";

    var 状态图标盒 = document.createElement("span");
    状态图标盒.className = "状态图标";
    状态图标盒.appendChild(图标(状态图标id(步, 活跃)));
    主题行1.appendChild(状态图标盒);

    var 号 = document.createElement("span");
    号.className = "号";
    号.textContent = "步骤 " + (步.步骤号 || "·");
    主题行1.appendChild(号);

    主题.appendChild(主题行1);

    var 主题行2 = document.createElement("div");
    主题行2.className = "主题行2";
    主题行2.textContent = 步.标题 || "";
    主题.appendChild(主题行2);

    div.appendChild(主题);

    // 结论：耗时
    var 结论 = document.createElement("div");
    结论.className = "卡片结论";

    var 耗时 = document.createElement("span");
    耗时.className = "耗时";
    耗时.textContent = 耗时文本(步["耗时ms"] || 0);
    结论.appendChild(耗时);

    div.appendChild(结论);
    return div;
  }

  // 并行块（降级形态）
  function 渲染并行块(线事件们) {
    var div = document.createElement("div");
    div.className = "并行块";
    线事件们.forEach(function (项) {
      var 卡 = document.createElement("div");
      卡.className = "线卡";
      var 头 = document.createElement("div");
      头.className = "线头";
      头.innerHTML = '<span class="线id">' + (项.线id || "?") + "</span>"
        + '<span class="角色">' + (项.角色 || "") + "</span>";
      卡.appendChild(头);
      var 体 = document.createElement("div");
      体.className = "线体";
      体.textContent = "▸ " + (项.动作 || "") + (项.完成 ? " ✓" : "");
      卡.appendChild(体);
      div.appendChild(卡);
    });
    return div;
  }

  // 汇合面板（矛盾清单）
  function 渲染汇合面板(矛盾清单) {
    var div = document.createElement("div");
    div.className = "汇合面板";

    var 头 = document.createElement("div");
    头.className = "头";
    var 头图标 = document.createElement("span");
    头图标.className = "头图标";
    头图标.appendChild(图标("ic-conflict"));
    头.appendChild(头图标);
    var 头文 = document.createElement("span");
    头文.textContent = "汇合 · 矛盾清单（" + 矛盾清单.length + "）";
    头.appendChild(头文);
    div.appendChild(头);

    矛盾清单.forEach(function (m) {
      var 项 = document.createElement("div");
      项.className = "矛盾项";
      项.innerHTML = '<span class="对峙">' + (m.准圣A || "?") + " vs " + (m.准圣B || "?") + "</span> "
        + (m.描述 || "");
      div.appendChild(项);
    });
    return div;
  }

  // 普通事件（任务树视图兼容）
  function 渲染事件(e) {
    var div = document.createElement("div");
    div.className = "事件 " + 态类(e.动作);
    if (e.动作 && (e.动作.indexOf("失败") >= 0 || e.动作.indexOf("⚠") >= 0)) div.classList.add("失败");
    if (e.动作 && (e.动作.indexOf("验收") >= 0 || e.动作.indexOf("版本") >= 0)) div.classList.add("重点");

    var 行0 = document.createElement("div");
    行0.className = "行0";
    行0.innerHTML = '<span class="ts">' + 格式时刻(e.ts) + "</span>"
      + '<span class="源">' + (e.源 || e.source || "") + "</span>"
      + '<span class="动作">' + (e.动作 || "") + "</span>"
      + '<span class="token">tk ' + ((e.token && e.token.总计) || 0) + "</span>"
      + '<span class="耗时">' + 耗时文本(e["耗时ms"] || 0) + "</span>";
    div.appendChild(行0);

    var 详情 = document.createElement("div");
    详情.className = "详情";
    if (e.影响 && e.影响.length) {
      e.影响.forEach(function (项) {
        var p = document.createElement("div");
        p.className = "影响项";
        p.textContent = "· " + 项.类型 + ":" + 项.名 + (项.变化 ? " " + 项.变化 : "");
        详情.appendChild(p);
      });
    }
    if (e.证据) {
      var p = document.createElement("div");
      p.className = "证据";
      p.textContent = String(e.证据).slice(0, 800);
      详情.appendChild(p);
    }
    div.appendChild(详情);
    行0.addEventListener("click", function () { div.classList.toggle("展开"); });
    return div;
  }

  // 并发度指示
  function 刷新并发度() {
    var n = state.活跃线.size;
    var el = $("并发度");
    el.textContent = "线 " + n;
    el.classList.toggle("高亮", n >= 2);
    el.classList.toggle("零", n === 0);
  }

  // 旁路栏
  function 刷新旁路栏() {
    var 栏 = $("旁路栏");
    if (!state.旁路产出 || state.旁路产出.length === 0) {
      栏.style.display = "none";
      return;
    }
    栏.style.display = "";
    栏.innerHTML = "";
    var 头 = document.createElement("div");
    头.className = "头";
    var 头图标 = document.createElement("span");
    头图标.className = "头图标";
    头图标.appendChild(图标("ic-moon"));
    头.appendChild(头图标);
    var 头文 = document.createElement("span");
    头文.textContent = "旁路 · 阴脑";
    头.appendChild(头文);
    栏.appendChild(头);

    state.旁路产出.slice(-30).forEach(function (项) {
      var d = document.createElement("div");
      d.className = "阴脑项";
      var ts = document.createElement("span");
      ts.className = "ts";
      ts.textContent = 格式时刻(项.ts);
      d.appendChild(ts);
      var 内容 = document.createElement("div");
      内容.className = "内容";
      内容.textContent = (项.动作 || 项.摘要 || "");
      d.appendChild(内容);
      栏.appendChild(d);
    });
    栏.scrollTop = 栏.scrollHeight;
  }

  // 页脚（图标 + 数值 + 标签）
  function 刷新页脚() {
    var 页 = $("页脚");
    页.innerHTML = "";

    function 统计项(图标id, 数值, 标签) {
      var 项 = document.createElement("span");
      项.className = "统计项";
      var 图盒 = document.createElement("span");
      图盒.className = "统计图标";
      图盒.appendChild(图标(图标id));
      项.appendChild(图盒);
      var 数 = document.createElement("span");
      数.className = "统计数值";
      数.textContent = 数值;
      项.appendChild(数);
      var 签 = document.createElement("span");
      签.className = "统计标签";
      签.textContent = 标签;
      项.appendChild(签);
      页.appendChild(项);
    }
    统计项("ic-events", state.统计.事件数, "事件");
    统计项("ic-token", state.统计.token, "token");
    统计项("ic-clock", 耗时文本(state.统计.耗时ms), "耗时");
    统计项("ic-lines", state.活跃线.size, "活跃线");
  }

  // 分裂/合流检测
  function 检测分裂合流(旧数, 新数, ts) {
    if (旧数 <= 1 && 新数 >= 2) {
      state.分裂点.push({ ts: ts, 类型: "分裂" });
    } else if (旧数 >= 2 && 新数 <= 1) {
      state.分裂点.push({ ts: ts, 类型: "合流" });
    }
  }

  // 收集并行批
  function 收集并行批() {
    var 批 = [];
    state.活跃线.forEach(function (线id) {
      for (var i = state.事件池.length - 1; i >= 0; i--) {
        if (取线id(state.事件池[i]) === 线id) {
          var e = state.事件池[i];
          批.push({
            线id: 线id,
            角色: e.角色 || "",
            动作: e.动作,
            完成: e.动作 && e.动作.indexOf("✓") >= 0
          });
          break;
        }
      }
    });
    return 批;
  }

  function 渲染流增量(e, 线id, 活跃数) {
    if (活跃数 <= 1 || !线id) {
      渲染步骤流全量();
    } else {
      state.并行块 = 收集并行批();
      渲染步骤流全量();
    }
  }

  // 处理单条事件
  function 处理事件(e) {
    state.事件池.push(e);
    if (state.事件池.length > MAX) state.事件池.shift();
    state.统计.事件数++;
    if (e.token) state.统计.token += (e.token.总计 || 0);
    state.统计.耗时ms += (e["耗时ms"] || 0);

    var 线id = 取线id(e);
    var 旧数 = state.活跃线.size;
    if (线id) {
      state.活跃线.add(线id);
      if (e.动作 && (e.动作.indexOf("完成") >= 0 || e.动作.indexOf("结束") >= 0 || e.动作.indexOf("✓") >= 0)) {
        state.活跃线.delete(线id);
      }
    }
    var 新数 = state.活跃线.size;

    var 源 = e.源 || e.source || "";
    if (源.indexOf("阴") >= 0 || 源.indexOf("旁路") >= 0) {
      state.旁路产出.push({ ts: e.ts, 动作: e.动作, 摘要: e.摘要 });
      if (state.旁路产出.length > 100) state.旁路产出.shift();
      刷新旁路栏();
    }

    if (state.视图 === "步骤流") {
      检测分裂合流(旧数, 新数, e.ts);
      渲染流增量(e, 线id, 新数);
    }
    刷新并发度();
    刷新页脚();
  }

  // 任务列表（按任务线id分组）
  function 刷新任务列表() {
    fetch("/api/tasks").then(function (r) { return r.json(); }).then(function (idx) {
      var 列表 = $("任务列表");
      列表.innerHTML = "";
      var 线组 = {};
      var 任务们 = idx.任务 || [];
      任务们.forEach(function (t) {
        var 线id = t.任务线id || t.线.id || "主线";
        if (!线组[线id]) 线组[线id] = [];
        线组[线id].push(t);
      });
      Object.keys(线组).forEach(function (线id) {
        var 头 = document.createElement("div");
        头.className = "线组头";
        头.textContent = "线 " + 线id;
        列表.appendChild(头);
        线组[线id].forEach(function (t) {
          var div = document.createElement("div");
          div.className = "任务" + (t.id === state.选中任务线 ? " 选中" : "");

          var 态圆 = document.createElement("span");
          态圆.className = "态圆 " + 态圆类(t.状态);
          div.appendChild(态圆);

          var 摘要 = document.createElement("span");
          摘要.className = "摘要";
          摘要.textContent = t.id + " " + (t.摘要 || "").slice(0, 20);
          div.appendChild(摘要);

          div.addEventListener("click", function () {
            state.选中任务线 = t.id;
            加载线步骤流(线id);
            刷新任务列表();
          });
          列表.appendChild(div);
        });
      });
    }).catch(function (err) { console.error("tasks加载失败", err); });
  }

  // 任务态圆点类名
  function 态圆类(状态) {
    if (!状态) return "其他";
    if (状态.indexOf("完成") >= 0 || 状态.indexOf("✓") >= 0) return "完成";
    if (状态.indexOf("进行") >= 0 || 状态.indexOf("跑") >= 0 || 状态.indexOf("活") >= 0) return "进行";
    if (状态.indexOf("等待") >= 0 || 状态.indexOf("挂") >= 0) return "等待";
    if (状态.indexOf("失败") >= 0 || 状态.indexOf("错") >= 0) return "失败";
    return "其他";
  }

  function 加载线步骤流(线id) {
    fetch("/api/lines/" + encodeURIComponent(线id) + "/steps").then(function (r) { return r.json(); }).then(function (步骤们) {
      state.步骤流[线id] = 步骤们 || [];
      渲染线步骤流(线id);
    }).catch(function (err) { console.error("线步骤流加载失败", err); });
  }

  function 渲染线步骤流(线id) {
    var 流div = $("事件流");
    流div.innerHTML = "";
    var 头 = document.createElement("div");
    头.className = "线组头";
    头.textContent = "线 " + 线id + " 步骤流";
    流div.appendChild(头);
    var 步骤们 = state.步骤流[线id] || [];
    步骤们.forEach(function (步, i) {
      var 活跃 = i === 步骤们.length - 1 && !步.完成;
      流div.appendChild(渲染步骤(步, 活跃));
    });
  }

  // 视图切换
  function 切换视图() {
    document.querySelectorAll(".段头 button").forEach(function (b) {
      b.classList.toggle("激活", b.dataset.view === state.视图);
    });
    var 流div = $("事件流");
    流div.innerHTML = "";
    if (state.视图 === "步骤流") {
      渲染步骤流全量();
    } else {
      fetch("/api/tasks").then(function (r) { return r.json(); }).then(function (idx) {
        (idx.任务 || []).forEach(function (t) {
          var div = document.createElement("div");
          div.className = "事件 重点";
          div.innerHTML = '<div class="行0"><span class="动作">' + t.id + ": " + (t.摘要 || "") + "</span>"
            + '<span class="token">事件 ' + (t.事件数 || 0) + " tk " + (t.累计token || 0) + "</span>"
            + '<span class="耗时">' + 耗时文本(t.累计耗时ms || 0) + "</span></div>";
          流div.appendChild(div);
        });
      }).catch(function (err) { console.error("任务树加载失败", err); });
    }
  }

  // SSE 订阅
  function 订阅() {
    if (流) 流.close();
    流 = new EventSource("/api/events/stream");
    流.onmessage = function (ev) {
      try {
        var 载荷 = JSON.parse(ev.data);
        处理事件(载荷.ev || 载荷);
      } catch (err) {}
    };
    流.onerror = function () {
      $("状态点").style.background = "var(--err)";
      流.close();
      setTimeout(function () { 订阅(); }, 3000);
    };
    流.onopen = function () { $("状态点").style.background = "var(--ok)"; };
  }

  // 初始化
  function 初始化() {
    document.querySelectorAll(".段头 button").forEach(function (b) {
      b.addEventListener("click", function () { state.视图 = b.dataset.view; 切换视图(); });
    });
    fetch("/api/snapshot").then(function (r) { return r.json(); }).then(function (s) {
      $("当前要求").textContent = s.当前要求 || "";
      $("阶段").textContent = s.当前阶段 || "";
      $("当前想法").textContent = s.当前想法 || "";
      state.矛盾清单 = s.矛盾清单 || null;
    }).catch(function (err) { console.error("snapshot加载失败", err); });
    fetch("/api/topology").then(function (r) { return r.json(); }).then(function (t) {
      state.拓朴 = t || [];
    }).catch(function (err) { console.error("topology加载失败", err); });
    fetch("/api/events/recent?limit=200").then(function (r) { return r.json(); }).then(function (evs) {
      (evs || []).forEach(function (e) { 处理事件(e); });
    }).catch(function (err) { console.error("recent事件加载失败", err); }).finally(function () {
      刷新任务列表();
      订阅();
    });
    setInterval(function () {
      $("时刻").textContent = new Date().toLocaleTimeString("zh-CN", { hour12: false });
    }, 1000);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", 初始化);
  } else {
    初始化();
  }
})();
