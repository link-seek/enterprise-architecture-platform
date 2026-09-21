# Vendored 模板目录（构建上下文内副本）

`pilot-consumer/` 是仓库根目录 `templates/pilot-consumer/` 的**逐文件副本**，
唯一原因是：后端镜像的 Docker 构建上下文是 `backend/`，根目录的
`templates/` 进不了 builder，更进不了 distroless 运行镜像。
`backend/Dockerfile` 运行阶段会 `COPY --from=builder /app/templates ./templates`，
与 `pilot.rs::template_dir()` 的兜底路径 `/app/templates/pilot-consumer` 对齐。

修改模板时请**两处同改**（根目录为准），然后重新核对：

```sh
diff -r ../../templates/pilot-consumer pilot-consumer && echo OK
```
