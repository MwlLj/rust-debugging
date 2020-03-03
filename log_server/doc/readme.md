[TOC]

## 启动
- 参数
    - -port: 监听端口
    - -host: 监听ip
    - -thread-max: 最大线程数, 默认 10
    - -max-day: 日志保留的最大天数, 默认 5
- 命令
    - ./log_server [-port [port]] [-host [host]] [-thread-max [threadmax]] [-max-day [maxday]]
- 例
    - ./log_server -port 51000 -host 0.0.0.0 -thread-max 10 -max-day 5
