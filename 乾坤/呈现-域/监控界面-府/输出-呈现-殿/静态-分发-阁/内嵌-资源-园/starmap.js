// starmap.js · §13.f.10.3b 函数级调用图谱·星空视图
(function () {
  'use strict';

  var 状态 = {
    节点们: [],
    边们: [],
    布局: new Map(),   // id -> {x, y, vx, vy}
    暂停: false,
    只显示有入度: true,
    svg: null,
    连线层: null,
    节点层: null,
    选中: null,
    宽度: 1200,
    高度: 800,
  };

  var crate颜色 = {
    '识海承载-府': '#5c6bc0',
    '天庭治理-府': '#13d4a4',
    '道术施展-府': '#aed581',
    '模型连接-府': '#4fc3f7',
    '日志记录-府': '#ff9800',
    '观测探针-府': '#ef5350',
    '配置管理-府': '#7e57c2',
    '插件承载-府': '#ba68c8',
    '状态共享-府': '#f06292',
    '事件总线-府': '#ffb74d',
    '命令操作-府': '#ffd54f',
    '监控界面-府': '#4dd0e1',
    '单元测试-府': '#a1887f',
    'unknown': '#666',
  };

  function 加载() {
    fetch('/api/starmap').then(function (r) { return r.json(); }).then(function (data) {
      状态.节点们 = data.节点们 || [];
      状态.边们 = data.边们 || [];
      渲染统计();
      初始化布局();
      开始模拟();
      状态.svg = document.getElementById('星图舞台');
      状态.连线层 = document.getElementById('连线层');
      状态.节点层 = document.getElementById('节点层');
      绑定交互();
      状态.svg.setAttribute('viewBox', '0 0 ' + 状态.宽度 + ' ' + 状态.高度);
      document.getElementById('状态点').classList.add('就绪');
      document.getElementById('状态文').textContent = '星辰大海已就位';
      渲染();
    }).catch(function (e) {
      document.getElementById('状态文').textContent = '加载失败: ' + e;
    });
  }

  function 渲染统计() {
    var 显示节点 = 状态.只显示有入度 ? 状态.节点们.filter(function (n) { return n.大小 > 1; }) : 状态.节点们;
    document.getElementById('统计').textContent = 显示节点.length + ' 节点 / ' + 状态.边们.length + ' 边（总 ' + 状态.节点们.length + '）';
  }

  function 初始化布局() {
    var 显示节点 = 当前显示节点();
    var 半径 = Math.min(状态.宽度, 状态.高度) * 0.4;
    var 中心x = 状态.宽度 / 2, 中心y = 状态.高度 / 2;
    显示节点.forEach(function (n, i) {
      var 角度 = (i / 显示节点.length) * Math.PI * 2;
      状态.布局.set(n.id, {
        x: 中心x + Math.cos(角度) * 半径 * (0.5 + Math.random() * 0.5),
        y: 中心y + Math.sin(角度) * 半径 * (0.5 + Math.random() * 0.5),
        vx: 0, vy: 0,
      });
    });
  }

  function 当前显示节点() {
    return 状态.只显示有入度 ? 状态.节点们.filter(function (n) { return n.大小 > 1; }) : 状态.节点们;
  }

  function 开始模拟() {
    var 迭代步 = 0;
    var 最大步 = 80;
    function 步进() {
      if (迭代步 < 最大步 && !状态.暂停) {
        迭代();
        渲染();
        迭代步++;
      }
      requestAnimationFrame(步进);
    }
    function 迭代() {
      var 显示节点 = 当前显示节点();
      var 节点by_id = new Map();
      显示节点.forEach(function (n) { 节点by_id.set(n.id, n); });

      // 斥力
      for (var i = 0; i < 显示节点.length; i++) {
        for (var j = i + 1; j < 显示节点.length; j++) {
          var a = 状态.布局.get(显示节点[i].id);
          var b = 状态.布局.get(显示节点[j].id);
          if (!a || !b) continue;
          var dx = b.x - a.x, dy = b.y - a.y;
          var dist2 = dx * dx + dy * dy + 0.01;
          var dist = Math.sqrt(dist2);
          var 力 = 5000 / dist2;
          a.vx -= (dx / dist) * 力; a.vy -= (dy / dist) * 力;
          b.vx += (dx / dist) * 力; b.vy += (dy / dist) * 力;
        }
      }

      // 引力（边）
      状态.边们.forEach(function (e) {
        var a = 状态.布局.get(e.源);
        var b = 状态.布局.get(e.目标);
        if (!a || !b) return;
        var dx = b.x - a.x, dy = b.y - a.y;
        var dist = Math.sqrt(dx * dx + dy * dy) + 0.01;
        var 力 = dist * 0.001;
        a.vx += (dx / dist) * 力; a.vy += (dy / dist) * 力;
        b.vx -= (dx / dist) * 力; b.vy -= (dy / dist) * 力;
      });

      // 应用速度 + 阻尼
      显示节点.forEach(function (n) {
        var p = 状态.布局.get(n.id);
        if (!p) return;
        p.vx *= 0.85; p.vy *= 0.85;
        p.x += p.vx; p.y += p.vy;
        p.x = Math.max(20, Math.min(状态.宽度 - 20, p.x));
        p.y = Math.max(20, Math.min(状态.高度 - 20, p.y));
      });
    }
    requestAnimationFrame(步进);
  }

  function 渲染() {
    if (!状态.连线层) return;
    var 显示节点 = 当前显示节点();
    var 显示id集 = new Set(显示节点.map(function (n) { return n.id; }));

    // 渲染边
    var 边html = '';
    状态.边们.forEach(function (e) {
      if (!显示id集.has(e.源) || !显示id集.has(e.目标)) return;
      var a = 状态.布局.get(e.源);
      var b = 状态.布局.get(e.目标);
      if (!a || !b) return;
      var 距离 = Math.sqrt((b.x - a.x) ** 2 + (b.y - a.y) ** 2);
      var 透明度 = Math.max(0.05, 1 - 距离 / 600);
      边html += '<line x1="' + a.x.toFixed(1) + '" y1="' + a.y.toFixed(1) + '" x2="' + b.x.toFixed(1) + '" y2="' + b.y.toFixed(1) + '" stroke="#4fc3f7" stroke-opacity="' + 透明度.toFixed(2) + '" stroke-width="0.5"/>';
    });
    状态.连线层.innerHTML = 边html;

    // 渲染节点
    var 节点html = '';
    显示节点.forEach(function (n) {
      var p = 状态.布局.get(n.id);
      if (!p) return;
      var 半径 = 2 + Math.min(8, n.大小);
      var 颜色 = crate颜色[n.crate_名] || crate颜色['unknown'];
      var 选中标记 = 状态.选中 === n.id ? ' stroke="#fff" stroke-width="2"' : '';
      节点html += '<g class="星点" data-id="' + escapeAttr(n.id) + '" transform="translate(' + p.x.toFixed(1) + ',' + p.y.toFixed(1) + ')">';
      节点html += '<circle r="' + (半径 + 4) + '" fill="url(#节点光晕-紫)" opacity="0.4"/>';
      节点html += '<circle r="' + 半径 + '" fill="' + 颜色 + '"' + 选中标记 + '/>';
      if (n.大小 >= 5) {
        节点html += '<text x="' + (半径 + 6) + '" y="3" font-size="10" fill="#e8e8e8" font-family="monospace">' + escapeHtml(n.名字) + '</text>';
      }
      节点html += '</g>';
    });
    状态.节点层.innerHTML = 节点html;
  }

  function 绑定交互() {
    // 点击节点
    状态.节点层.addEventListener('click', function (e) {
      var g = e.target.closest('.星点');
      if (g) {
        var id = g.dataset.id;
        状态.选中 = id;
        var n = 状态.节点们.find(function (x) { return x.id === id; });
        if (n) 显示详情(n);
        渲染();
      }
    });

    // 重置
    document.getElementById('重置').addEventListener('click', function () {
      初始化布局();
    });

    // 暂停
    document.getElementById('暂停').addEventListener('click', function () {
      状态.暂停 = !状态.暂停;
      this.textContent = 状态.暂停 ? '继续动画' : '暂停动画';
    });

    // 过滤
    document.getElementById('只显示有入度').addEventListener('change', function (e) {
      状态.只显示有入度 = e.target.checked;
      渲染统计();
      初始化布局();
      渲染();
    });

    // 关闭详情
    document.getElementById('详情-关闭').addEventListener('click', function () {
      状态.选中 = null;
      document.getElementById('详情面板').classList.add('隐藏');
      渲染();
    });

    // 拖动节点
    var 拖中 = null;
    状态.svg.addEventListener('mousedown', function (e) {
      var g = e.target.closest('.星点');
      if (g) {
        拖中 = g.dataset.id;
        e.preventDefault();
      }
    });
    状态.svg.addEventListener('mousemove', function (e) {
      if (!拖中) return;
      var rect = 状态.svg.getBoundingClientRect();
      var p = 状态.布局.get(拖中);
      if (p) {
        p.x = (e.clientX - rect.left) * (状态.宽度 / rect.width);
        p.y = (e.clientY - rect.top) * (状态.高度 / rect.height);
        p.vx = 0; p.vy = 0;
        渲染();
      }
    });
    状态.svg.addEventListener('mouseup', function () { 拖中 = null; });
  }

  function 显示详情(n) {
    var 面板 = document.getElementById('详情面板');
    document.getElementById('详情-名字').textContent = n.名字 + ' [' + n.类型 + ']';
    document.getElementById('详情-签名').textContent = 'crate: ' + n.crate_名;
    document.getElementById('详情-文件').textContent = n.文件;
    document.getElementById('详情-入度').textContent = '入度: ' + n.大小 + '（被调用次数）';
    var 出度 = 状态.边们.filter(function (e) { return e.源 === n.id; }).length;
    document.getElementById('详情-出度').textContent = '出度: ' + 出度 + '（调用了其他）';
    面板.classList.remove('隐藏');
  }

  function escapeHtml(s) {
    return String(s).replace(/[&<>"']/g, function (c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]; });
  }
  function escapeAttr(s) {
    return String(s).replace(/"/g, '&quot;');
  }

  加载();
})();
