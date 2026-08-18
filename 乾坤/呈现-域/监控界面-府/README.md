# 监控界面 · 第十府

> 落位:乾坤/呈现-域/监控界面-府
> 依据:融合蓝图-设计稿.md §11
> 本质:世界执行期间的直播 + 白箱 + 让用户信任

---

## 启动

    python server.py 8080

默认端口 8080。可以传参改端口:

    python server.py 9090

打开浏览器:http://127.0.0.1:8080

---

## 架构

    监控界面-府/
    ├── server.py             # Python 标准库 http.server + ThreadingHTTPServer
    ├── monitor.rooms.json   # 9 卡片清单与关切字段
    ├── README.md            # 本文件
    └── static/
        ├── index.html       # 主页(不到 50 行)
        ├── style.css        # 暗色主题 + 响应式
        └── app.js           # 直播 + 回放 + 设置(不到 200 行)

全部代码仅依赖 Python 标准库,零第三方依赖。

---

## 7 个端点

| 方法 | 路径 | 形态 | 用途 |
|:--|:--|:--|:--|
| GET | / | HTML | 主页 |
| GET | /static/* | 静态 | 资源 |
| GET | /api/snapshot | JSON | 当前快照(首帧 + 重连保护) |
| GET | /api/stream | SSE 长连 | 直播(服务端持续推送新事件) |
| GET | /api/replay?since=&until=&府= | NDJSON | 回放(按时间窗重放历史) |
| GET | /api/rooms | JSON | 9 卡片清单 |
| GET/POST | /api/settings | JSON | 读/写设置(唯一允许的写) |
| GET | /api/health | JSON | 健康检查 |

---

## 事件结构(白箱六字段,§9.3)

每条进入直播的事件必须是以下结构:

    {
      ts: 1787038400123,
      源: '识海/格位:调用',
      动作: '铭记·写入格位 <<调用>>',
      影响: [{类型: '格位', 名: '调用', 字节: 189000}],
      token: {提示词: 0, 输出: 0, 缓存: 0, 总计: 0},
      耗时ms: 0,
      证据: ''
    }

缺一即白箱泄漏 —— 本版由 server.py 的 transform 装配器守护,原始行缺字段填 0 / '',不驱回。

---

## 依据

- 融合蓝图-设计稿.md §11 §6.2 §9.2 §9.3 §8.1 §8.2 §11.5.3
- AGENTS.md §5:不重复实现,数据从 .context/观测/记录.jsonl 读不另造
- AGENTS.md §6:初始化与报错经 rizhi_fu(本版以 print 临时踩着调用点,未来可换为调 lib)

---

## 未来

界主: 'python先实现,然后我看看,再说转rust'
本版是 Python 跳板 —— 验证体验、调试接口、收集使用者反馈后,再决定是否重写为 Rust 实现(引入 axum + tokio + SSE)。
