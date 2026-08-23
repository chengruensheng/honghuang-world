// trajectory.js · §13.f 时序·历史视图
// 对标 Chrome Network 面板：表格行 + Turn 分组 + 7 种事件类型 + 思考折叠
// 数据源：/api/events/recent（历史）+ /api/events/stream（实时）

(function () {
  'use strict';

  // ===== 状态 =====
  var 状态 = {
    事件们: [],          // 所有事件（含 mock + 真实）
    折叠Turn们: new Set(),
    折叠消息们: new Set(),
    搜索关键词: '',
    事件源: null,         // EventSource
    流速计数: { 起始毫秒: Date.now(), 累计条数: 0 },
    类型映射: {},          // id → 类型
    道韵数据: null,        // §十三 道韵扫描结果（候选池 + 法则违逆）
    道韵面板开: false,    // 面板是否展开
  };

  // URL hash 折叠/搜索持久化：§13.f.6 沿用
  // 格式：#trajectory?turn=TL-001,TL-002&msg=id-abc,id-xyz&search=关键词
  function 保存折叠到hash() {
    try {
      var turns = Array.from(状态.折叠Turn们).join(',');
      var msgs = Array.from(状态.折叠消息们).join(',');
      var parts = [];
      if (turns) parts.push('turn=' + turns);
      if (msgs) parts.push('msg=' + msgs);
      if (状态.搜索关键词) parts.push('search=' + encodeURIComponent(状态.搜索关键词));
      var hash = parts.length ? '#trajectory?' + parts.join('&') : '#trajectory';
      if (location.hash !== hash) history.replaceState(null, '', hash);
    } catch (e) { console.warn('hash 持久化失败', e); }
  }

  function 从hash恢复折叠() {
    try {
      var hash = location.hash;
      if (!hash.startsWith('#trajectory')) return;
      var query = hash.split('?')[1] || '';
      if (!query) return;
      query.split('&').forEach(function (kv) {
        var i = kv.indexOf('=');
        if (i < 0) return;
        var k = kv.substring(0, i);
        var v = decodeURIComponent(kv.substring(i + 1) || '');
        if (k === 'turn') v.split(',').forEach(function (t) { if (t) 状态.折叠Turn们.add(t); });
        else if (k === 'msg') v.split(',').forEach(function (m) { if (m) 状态.折叠消息们.add(m); });
        else if (k === 'search') {
          状态.搜索关键词 = v;
          var sb = document.getElementById('搜索框');
          if (sb) sb.value = v;
        }
      });
    } catch (e) { console.warn('hash 恢复失败', e); }
  }

  // 搜索高亮：转义 HTML + 包高亮标签
function 高亮(escaped, 关键词) {
  if (!关键词) return escaped;
  try {
    var kw = 关键词.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    var re = new RegExp('(' + kw + ')', 'gi');
    return escaped.replace(re, '<mark class="高亮">$1</mark>');
  } catch (e) { return escaped; }
}
// ===== 7 种事件类型派生（§13.f.8）=====
  function 派生类型(事件) {
    var 源 = (事件 && 事件.源) || '';
    var 动作 = (事件 && 事件.动作) || '';
    if (源.indexOf('提示词') >= 0) {
      // 提示词域细分：系统 vs 界主（按载荷角色字段）
      var 载荷 = 事件.载荷 || {};
      var 附加 = 载荷.附加 || {};
      if (附加.角色 === '系统' || 附加.角色 === 'system') return 'system';
      if (附加.角色 === '界主' || 附加.角色 === 'user') return 'user';
      return 'context'; // 提示词域兜底为 context
    }
    if (动作.indexOf('回复') >= 0 || 动作.indexOf('思考') >= 0 || 源.indexOf('模型连接') >= 0 || 源.indexOf('回复思考') >= 0) {
      return 'message';
    }
    if (动作.indexOf('压缩') >= 0) return 'compacted';
    if (动作.indexOf('工具调用') >= 0 || 源.indexOf('工具调用') >= 0) return 'tool';
    if (动作.indexOf('工具返回') >= 0 || 源.indexOf('工具返回') >= 0) return 'tool';
    if (动作.indexOf('子工具') >= 0) return 'subtool';
    return 'context'; // 兜底
  }

  // 类型中文标签 + CSS class
  var 类型标签 = {
    system: '系统',
    user: '界主',
    context: '上下文',
    compacted: '压缩',
    message: '消息',
    tool: '工具',
    subtool: '子工具',
  };

  // ===== 工具函数 =====
  function 转时间(ts) {
    var d = new Date(ts);
    var h = String(d.getHours()).padStart(2, '0');
    var m = String(d.getMinutes()).padStart(2, '0');
    var s = String(d.getSeconds()).padStart(2, '0');
    var ms = String(d.getMilliseconds()).padStart(3, '0');
    return h + ':' + m + ':' + s + '.' + ms;
  }

  function 转耗时(ms) {
    if (!ms || ms < 1) return '<1ms';
    if (ms < 1000) return ms + 'ms';
    if (ms < 60000) return (ms / 1000).toFixed(1) + 's';
    return Math.floor(ms / 60000) + 'm' + Math.floor((ms % 60000) / 1000) + 's';
  }

  function 转Token(n) {
    if (!n || n === 0) return '·';
    if (n >= 1000) return (n / 1000).toFixed(1) + 'k';
    return String(n);
  }

  function 取角色(源) {
    var parts = 源.split('·');
    return parts[parts.length - 1] || '未知';
  }

  function 取摘要(事件) {
    var 动作 = (事件 && 事件.动作) || '';
    var 源 = (事件 && 事件.源) || '';
    var 载荷 = (事件 && 事件.载荷) || {};
    var 证据 = 事件.证据 || '';
    // 工具调用：显示工具名 + 参数摘要
    if (事件.类型 === 'tool' && 载荷.附加 && 载荷.附加.工具) {
      var args = 证据.length > 80 ? 证据.slice(0, 80) + '…' : 证据;
      return '【' + 载荷.附加.工具 + '】 ' + args;
    }
    // 工具返回：显示前 100 字
    if (动作.indexOf('工具返回') >= 0 || 源.indexOf('工具返回') >= 0) {
      return 证据.length > 100 ? 证据.slice(0, 100) + '…' : 证据;
    }
    // 回复/消息：显示回复前 120 字
    if (证据 && 证据.length > 0) {
      return 证据.length > 120 ? 证据.slice(0, 120) + '…' : 证据;
    }
    return 动作;
  }

  function 短哈希(s) {
    var h = 0;
    for (var i = 0; i < s.length; i++) h = ((h << 5) - h + s.charCodeAt(i)) | 0;
    return 'id-' + Math.abs(h).toString(36);
  }

  // ===== Turn 分组（按任务线或会话 id）=====
  function 分Turn们(事件们) {
    // 简化：按 任务线id 分 Turn；空任务线归到 "未分组"
    var 桶 = new Map();
    事件们.forEach(function (e) {
      var tid = e.任务线id || '未分组';
      if (!桶.has(tid)) 桶.set(tid, []);
      桶.get(tid).push(e);
    });
    var turns = [];
    var turn序号 = 0;
    桶.forEach(function (组事件, tid) {
      turn序号++;
      var 累计提示词 = 0, 累计输出 = 0, 累计推理 = 0, 累计总计 = 0, 累计耗时 = 0;
      组事件.forEach(function (e) {
        累计提示词 += (e.token && e.token.提示词) || 0;
        累计输出 += (e.token && e.token.输出) || 0;
        累计推理 += (e.token && e.token.推理) || 0;
        累计总计 += (e.token && e.token.总计) || 0;
        累计耗时 += e.耗时ms || 0;
      });
      turns.push({
        序号: turn序号,
        id: tid,
        事件们: 组事件,
        累计提示词: 累计提示词,
        累计输出: 累计输出,
        累计推理: 累计推理,
        累计总计: 累计总计,
        累计耗时: 累计耗时,
        起始ts: 组事件[0] ? 组事件[0].ts : 0,
      });
    });
    return turns;
  }

  // ===== 渲染 =====
  var 表体 = document.getElementById('表体');
  var 事件计数 = document.getElementById('事件计数');
  var 总览 = document.getElementById('总览');

  function 渲染全部() {
    var turns = 分Turn们(状态.事件们);
    表体.innerHTML = '';

    if (turns.length === 0) {
      表体.innerHTML = '<div class="加载">暂无事件</div>';
      事件计数.textContent = '0 事件';
      总览.textContent = '总计 0 事件 / 0 Turn';
      return;
    }

    var 总事件 = 0;
    var 显示Turns = turns;

    // 搜索过滤
    if (状态.搜索关键词) {
      var kw = 状态.搜索关键词.toLowerCase();
      显示Turns = turns.map(function (t) {
        return Object.assign({}, t, {
          事件们: t.事件们.filter(function (e) {
            var 串 = (e.动作 + ' ' + e.源 + ' ' + (e.证据 || '') + ' ' + JSON.stringify(e.影响 || [])).toLowerCase();
            return 串.indexOf(kw) >= 0;
          }),
        });
      }).filter(function (t) { return t.事件们.length > 0; });
    }

    显示Turns.forEach(function (turn) {
      // Turn 段头（sticky）
      var 折叠 = 状态.折叠Turn们.has(turn.id);
      var header = document.createElement('div');
      header.className = 'turn-header' + (折叠 ? ' 折叠' : '');
      header.innerHTML =
        '<span class="turn-折叠">▸</span>' +
        '<span class="turn-序号">Turn ' + turn.序号 + '</span>' +
        '<span class="turn-id">' + turn.id + '</span>' +
        '<span class="turn-起始">' + 转时间(turn.起始ts) + '</span>' +
        '<span class="turn-累计">总计 ' + 转Token(turn.累计总计) + ' tok · ' + 转耗时(turn.累计耗时) + '</span>';
      header.addEventListener('click', function () { 切折叠Turn(turn.id); });
      表体.appendChild(header);

      if (!折叠) {
        // Turn 内事件
        turn.事件们.forEach(function (事件) {
          总事件++;
          事件.类型 = 派生类型(事件);
          事件.序号 = 总事件;
          表体.appendChild(渲染行(事件));
        });
      }
    });

    事件计数.textContent = 状态.事件们.length + ' 事件';
    总览.textContent = '总计 ' + 状态.事件们.length + ' 事件 / ' + turns.length + ' Turn';
  }

  // §十三.b 道韵面板渲染：候选池 + 法则违逆（按优先级/严重度排序）
  function 渲染道韵面板() {
    var 面板 = document.getElementById('道韵面板');
    if (!面板) return;
    if (!状态.道韵数据) {
      面板.innerHTML = '<div class="道韵空">道韵数据加载中……</div>';
      return;
    }
    var 候选们 = (状态.道韵数据.巡世候选们 || []).slice();
    var 违逆们 = (状态.道韵数据.天道报告库 && 状态.道韵数据.天道报告库.length > 0
      ? (状态.道韵数据.天道报告库[状态.道韵数据.天道报告库.length - 1].违逆 || [])
      : []);
    var 优先级顺序 = { '高': 0, '中': 1, '低': 2 };
    var 严重度顺序 = { '错误': 0, '警告': 1 };
    候选们.sort(function (a, b) {
      var pa = 优先级顺序[a.优先级] !== undefined ? 优先级顺序[a.优先级] : 3;
      var pb = 优先级顺序[b.优先级] !== undefined ? 优先级顺序[b.优先级] : 3;
      return pa - pb;
    });
    违逆们.sort(function (a, b) {
      var sa = 严重度顺序[a.严重度] !== undefined ? 严重度顺序[a.严重度] : 2;
      var sb = 严重度顺序[b.严重度] !== undefined ? 严重度顺序[b.严重度] : 2;
      return sa - sb;
    });
    面板.className = '道韵面板' + (状态.道韵面板开 ? ' 展开' : ' 折叠');
    var html = '<div class="道韵头">' +
      '<span class="道韵标题">§十三 道韵违逆扫描</span>' +
      '<span class="道韵统计">候选 <b>' + 候选们.length + '</b> 条 · 法则违逆 <b>' + 违逆们.length + '</b> 条</span>' +
      '</div>';
    html += '<div class="道韵段"><h4>候选池（按优先级）</h4>';
    if (候选们.length === 0) {
      html += '<div class="道韵空">当前无候选</div>';
    } else {
      html += '<table class="道韵表"><thead><tr><th>优先级</th><th>类别</th><th>目标</th><th>依据</th></tr></thead><tbody>';
      var 显示候选 = 候选们.slice(0, 20);
      显示候选.forEach(function (c) {
        var 优先级类 = '道韵优先级-' + (c.优先级 || '');
        html += '<tr>' +
          '<td><span class="' + 优先级类 + '">' + escapeHtml(c.优先级 || '') + '</span></td>' +
          '<td>' + escapeHtml(c.建议类别 || '') + '</td>' +
          '<td>' + escapeHtml(c.目标 || '') + '</td>' +
          '<td class="道韵依据">' + escapeHtml((c.依据 || '').substring(0, 80)) + '</td>' +
          '</tr>';
      });
      html += '</tbody></table>';
      if (候选们.length > 20) {
        html += '<div class="道韵空">…还有 ' + (候选们.length - 20) + ' 条候选未显示</div>';
      }
    }
    html += '</div>';
    html += '<div class="道韵段"><h4>法则违逆（按严重度）</h4>';
    if (违逆们.length === 0) {
      html += '<div class="道韵空">✓ 当前无违逆（项目干净）</div>';
    } else {
      html += '<table class="道韵表"><thead><tr><th>严重度</th><th>路径</th><th>内容</th><th>依据</th></tr></thead><tbody>';
      违逆们.forEach(function (v) {
        var 严重度类 = '道韵严重度-' + (v.严重度 || '');
        html += '<tr>' +
          '<td><span class="' + 严重度类 + '">' + escapeHtml(v.严重度 || '') + '</span></td>' +
          '<td class="道韵路径">' + escapeHtml(v.路径 || '') + '</td>' +
          '<td>' + escapeHtml(v.违逆内容 || '') + '</td>' +
          '<td class="道韵依据">' + escapeHtml(v.依据规则 || '') + '</td>' +
          '</tr>';
      });
      html += '</tbody></table>';
    }
    html += '</div>';
    面板.innerHTML = html;
  }

  // §十三.b 道韵按钮：展开/折叠
  document.getElementById('道韵按钮').addEventListener('click', function () {
    状态.道韵面板开 = !状态.道韵面板开;
    渲染道韵面板();
  });

  function 渲染行(事件) {
    var 行 = document.createElement('div');
    行.className = '表行 类型-' + (事件.类型 || 'context');
    行.dataset.id = 短哈希(String(事件.ts) + '|' + (事件.动作 || ''));

    var token = 事件.token || {};
    var 摘要 = 取摘要(事件);
    摘要 = 高亮(摘要, 状态.搜索关键词);
    var 思考 = 事件.思考链 || '';
    var 思考长 = 思考.length;
    var 折叠消息 = 状态.折叠消息们.has(行.dataset.id);

    // 行内容
    var html =
      '<span class="列-序">' + 事件.序号 + '</span>' +
      '<span class="列-类型"><span class="类型标签 ' + 事件.类型 + '">' + (类型标签[事件.类型] || '其他') + '</span></span>' +
      '<span class="列-时间">' + 转时间(事件.ts) + '</span>' +
      '<span class="列-角色">' + 取角色(事件.源) + '</span>' +
      '<span class="列-摘要">' + escapeHtml(摘要) + '</span>' +
      '<span class="列-提示词">' + 转Token(token.提示词) + '</span>' +
      '<span class="列-输出">' + 转Token(token.输出) + '</span>' +
      '<span class="列-缓存">' + 转Token(token.缓存) + '</span>' +
      '<span class="列-缓存写">' + 转Token(token.缓存写) + '</span>' +
      '<span class="列-推理">' + 转Token(token.推理) + '</span>' +
      '<span class="列-总计">' + 转Token(token.总计) + '</span>' +
      '<span class="列-耗时">' + 转耗时(事件.耗时ms) + '</span>';

    行.innerHTML = html;

    // 思考折叠区
    if (思考 && 思考.trim().length > 0 && 事件.类型 === 'message') {
      var 按钮 = document.createElement('span');
      按钮.className = '思考按钮';
      按钮.textContent = 折叠消息 ? '▶ 思考 (' + 思考长 + '字)' : '▼ 思考';
      按钮.addEventListener('click', function (e) {
        e.stopPropagation();
        if (状态.折叠消息们.has(行.dataset.id)) {
          状态.折叠消息们.delete(行.dataset.id);
        } else {
          状态.折叠消息们.add(行.dataset.id);
        }
        保存折叠到hash();
        渲染全部();
      });
      行.querySelector('.列-摘要').appendChild(按钮);

      if (!折叠消息) {
        var 思考区 = document.createElement('div');
        思考区.className = '思考展开';
        // 高亮（设 innerHTML 而不是 textContent）
        思考区.innerHTML = 高亮(escapeHtml(思考), 状态.搜索关键词);
        行.appendChild(思考区);
      }
    }

    // 工具调用展开区
    if (事件.类型 === 'tool' && 事件.载荷 && 事件.载荷.附加 && 事件.载荷.附加.工具) {
      var 工具区 = document.createElement('div');
      工具区.className = '工具展开';
      工具区.innerHTML =
        '<div class="工具参数"><span class="标签">参数：</span>' + escapeHtml(事件.载荷.内容 || '') + '</div>';
      行.appendChild(工具区);
    }

    return 行;
  }

  function escapeHtml(s) {
    if (!s) return '';
    return String(s).replace(/[&<>"']/g, function (c) {
      return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c];
    });
  }

  function 切折叠Turn(id) {
    if (状态.折叠Turn们.has(id)) 状态.折叠Turn们.delete(id);
    else 状态.折叠Turn们.add(id);
    保存折叠到hash();
    渲染全部();
  }

  // ===== 数据加载 =====
  // 启动时从 URL hash 恢复折叠/搜索状态（在任何渲染前）
  从hash恢复折叠();

  function 加载道韵() {
    fetch('/api/daoyun').then(function (r) { return r.json(); }).then(function (数据) {
      状态.道韵数据 = 数据;
      渲染道韵面板();
    }).catch(function (e) {
      console.warn('加载道韵失败', e);
    });
  }

  function 加载历史() {
    fetch('/api/events/recent?n=200').then(function (r) { return r.json(); }).then(function (data) {
      if (Array.isArray(data)) {
        状态.事件们 = data;
        渲染全部();
      }
    }).catch(function (e) {
      console.error('加载历史失败', e);
      表体.innerHTML = '<div class="加载">加载失败：' + e + '</div>';
    });
  }

  加载道韵();

  function 启动SSE() {
    if (状态.事件源) 状态.事件源.close();
    状态.事件源 = new EventSource('/api/events/stream');
    状态.事件源.addEventListener('tick_event', function (e) {
      try {
        var 载荷 = JSON.parse(e.data);
        // 载荷形如 { source: "事件流"|"观测记录"|"识海记录", ts: ..., ev: 白箱事件 }
        var ev = 载荷.ev;
        if (!ev) return;
        // 标准化 id
        ev.类型 = 派生类型(ev);
        ev.序号 = 状态.事件们.length + 1;
        // 头插（按 ts 倒序）
        状态.事件们.unshift(ev);
        // 限制最大 500 条
        if (状态.事件们.length > 500) 状态.事件们.length = 500;
        渲染全部();
        更新流速();
      } catch (err) {
        console.error('SSE parse error', err);
      }
    });
    状态.事件源.onerror = function () {
      document.getElementById('状态点').classList.add('断开');
      document.getElementById('状态文').textContent = '信源断开';
    };
  }

  function 更新流速() {
    状态.流速计数.累计条数++;
    var 经过秒 = (Date.now() - 状态.流速计数.起始毫秒) / 1000;
    var 流速 = 经过秒 > 0 ? (状态.流速计数.累计条数 / 经过秒).toFixed(1) : '0';
    var el = document.getElementById('流速');
    if (el) el.textContent = '流速: ' + 流速 + ' ev/s';
  }

  // ===== 折叠全部/展开全部 =====
  document.getElementById('全部折叠').addEventListener('click', function () {
    状态.折叠Turn们 = new Set(分Turn们(状态.事件们).map(function (t) { return t.id; }));
    保存折叠到hash();
    渲染全部();
  });
  document.getElementById('全部展开').addEventListener('click', function () {
    状态.折叠Turn们 = new Set();
    状态.折叠消息们 = new Set();
    保存折叠到hash();
    渲染全部();
  });

  // ===== 搜索 =====
  document.getElementById('搜索框').addEventListener('input', function (e) {
    状态.搜索关键词 = e.target.value;
    保存折叠到hash();
    渲染全部();
  });

  // ===== 启动 =====
  加载历史();
  加载道韵();
  启动SSE();
  setInterval(更新流速, 1000);
})();

  // §十三.b 道韵面板渲染：候选池 + 法则违逆（按优先级/严重度排序）
  function 渲染道韵面板() {
    var 面板 = document.getElementById('道韵面板');
    if (!面板) return;
    if (!状态.道韵数据) {
      面板.innerHTML = '<div class="道韵空">道韵数据加载中……</div>';
      return;
    }
    var 候选们 = (状态.道韵数据.巡世候选们 || []).slice();
    var 违逆们 = (状态.道韵数据.天道报告库 && 状态.道韵数据.天道报告库.length > 0
      ? (状态.道韵数据.天道报告库[状态.道韵数据.天道报告库.length - 1].违逆 || [])
      : []);
    var 优先级顺序 = { '高': 0, '中': 1, '低': 2 };
    var 严重度顺序 = { '错误': 0, '警告': 1 };
    候选们.sort(function (a, b) {
      var pa = 优先级顺序[a.优先级] !== undefined ? 优先级顺序[a.优先级] : 3;
      var pb = 优先级顺序[b.优先级] !== undefined ? 优先级顺序[b.优先级] : 3;
      return pa - pb;
    });
    违逆们.sort(function (a, b) {
      var sa = 严重度顺序[a.严重度] !== undefined ? 严重度顺序[a.严重度] : 2;
      var sb = 严重度顺序[b.严重度] !== undefined ? 严重度顺序[b.严重度] : 2;
      return sa - sb;
    });

    // 面板样式
    面板.className = '道韵面板' + (状态.道韵面板开 ? ' 展开' : ' 折叠');

    // 顶部摘要
    var html = '<div class="道韵头">' +
      '<span class="道韵标题">§十三 道韵违逆扫描</span>' +
      '<span class="道韵统计">候选 <b>' + 候选们.length + '</b> 条 · 法则违逆 <b>' + 违逆们.length + '</b> 条</span>' +
      '</div>';

    // 候选列表
    html += '<div class="道韵段"><h4>候选池（按优先级）</h4>';
    if (候选们.length === 0) {
      html += '<div class="道韵空">当前无候选</div>';
    } else {
      html += '<table class="道韵表"><thead><tr><th>优先级</th><th>类别</th><th>目标</th><th>依据</th></tr></thead><tbody>';
      var 显示候选 = 候选们.slice(0, 20); // 限 20 条
      显示候选.forEach(function (c) {
        var 优先级类 = '道韵优先级-' + (c.优先级 || '');
        html += '<tr>' +
          '<td><span class="' + 优先级类 + '">' + escapeHtml(c.优先级 || '') + '</span></td>' +
          '<td>' + escapeHtml(c.建议类别 || '') + '</td>' +
          '<td>' + escapeHtml(c.目标 || '') + '</td>' +
          '<td class="道韵依据">' + escapeHtml((c.依据 || '').substring(0, 80)) + '</td>' +
          '</tr>';
      });
      html += '</tbody></table>';
      if (候选们.length > 20) {
        html += '<div class="道韵空">…还有 ' + (候选们.length - 20) + ' 条候选未显示</div>';
      }
    }
    html += '</div>';

    // 法则违逆
    html += '<div class="道韵段"><h4>法则违逆（按严重度）</h4>';
    if (违逆们.length === 0) {
      html += '<div class="道韵空">✓ 当前无违逆（项目干净）</div>';
    } else {
      html += '<table class="道韵表"><thead><tr><th>严重度</th><th>路径</th><th>内容</th><th>依据</th></tr></thead><tbody>';
      违逆们.forEach(function (v) {
        var 严重度类 = '道韵严重度-' + (v.严重度 || '');
        html += '<tr>' +
          '<td><span class="' + 严重度类 + '">' + escapeHtml(v.严重度 || '') + '</span></td>' +
          '<td class="道韵路径">' + escapeHtml(v.路径 || '') + '</td>' +
          '<td>' + escapeHtml(v.违逆内容 || '') + '</td>' +
          '<td class="道韵依据">' + escapeHtml(v.依据规则 || '') + '</td>' +
          '</tr>';
      });
      html += '</tbody></table>';
    }
    html += '</div>';

    面板.innerHTML = html;
  }

  // §十三.b 道韵面板按钮：展开/折叠
  document.getElementById('道韵按钮').addEventListener('click', function () {
    状态.道韵面板开 = !状态.道韵面板开;
    渲染道韵面板();
  });