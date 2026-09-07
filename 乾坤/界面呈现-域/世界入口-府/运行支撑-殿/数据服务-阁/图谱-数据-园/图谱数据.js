// 图谱数据.js —— 项目模块图谱（来自后端认知底座：模块集 / 符号集 / 依赖集 / 技术栈）

/** 从后端拉取项目图谱（模块集 / 符号集 / 依赖集 / 技术栈） */
export async function 加载图谱() {
  try {
    return await fetch('/api/cognition/graph').then((响应) => 响应.json());
  } catch (错误) {
    console.warn('加载图谱失败（后端未就绪？）', 错误);
    return { 模块集: [], 符号集: [], 依赖集: [], 技术栈: [] };
  }
}
