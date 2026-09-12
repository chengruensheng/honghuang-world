/* 映射注册：`node --import <本文件> --test` 时把解析钩子挂进模块解析链，
   使 `/xxx` 形式的 import 落到域根。门禁 校验前端测试覆盖.ps1 即以此运行。 */
import { register } from 'node:module';

register('./解析钩子.mjs', import.meta.url);
