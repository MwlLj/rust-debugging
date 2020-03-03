[TOC]

## 功能
- 监听 log_server客户端 发送的日志

## 监听
- 参数
    - -server: 服务地址(ip:port)
    - -server-name: 发送日志的服务名称
    - -server-version: 发送日志的服务版本
    - -server-no: 发送日志的服务编号
    - -keyword: 发送日志规定的内存关键字
- 命令
    - ./log_sub -server ip:port -server-name [name] -server-version [version] -server-no [no] -keyword [keyword]
- 例:
    - ./log_sub -server 192.168.9.128:51000 -server-name cfgs -server-version v2.0 -server-no 1 -keyword "mem1"
