## 改动摘要
<!-- 一句话说明 + 关键文件 -->

## 关联 Issue
<!-- closes #123 / 无 -->

## 自测记录
- [ ] 本地 `cargo test` 通过
- [ ] 本地 `npm run type-check` 零错误
- [ ] 本地 `npm run build` 通过
- [ ] 运行时手动验证（描述现象）：

## UI 变更截图（如有）

## 审查自检（对照 doc/09 §4）
- [ ] **Rust**：无临时值跨语句借用 / trait 已导入 / 锁粒度合理 / 平台代码用 `#[cfg(windows)]` 隔离
- [ ] **前端**：`vue-tsc` 零错误 / `invoke` 有错误处理 / 样式 `scoped`
- [ ] **隐私**：无网络外联 / 无密钥或证书入库
- [ ] **测试**：纯逻辑改动已补或更新 `#[cfg(test)]` 用例
- [ ] **提交**：符合 Conventional Commits，单 PR ≤ 400 行
