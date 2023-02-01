# mi_steps

![更新步数](https://github.com/dooooling/mi_steps/actions/workflows/rust.yml/badge.svg)
![github forks](https://badgen.net/github/forks/dooooling/mi_steps)
![github stars](https://badgen.net/github/stars/dooooling/mi_steps)

---
小米运动步数同步，支持邮箱同步、bark通知。

### 部署说明

#### 一、fork此项目

#### 二、设置账户信息

设置名为 ACCOUNTS 的 Secret，格式如下：

> 1234#1234</br>
> 5678#5678</br>
> 多账户注意换行 ！！！！

#### 三、设置bark（可选）

开启bark推送需要设置名为 ***BARK_SERVER*** 和 ***BARK_KEY*** 的Secret，格式如下：
> BARK_KEY safdafasfasf</br>
> BARK_SERVER https://api.day.app

#### 四、设置最大步数（可选）

自定义最大步数需要设置名为 ***MAX_STEPS*** 的Secret，默认为100000。
步数最大值的计算方式与最大值的同步时间相关，修改最大值的同步时间需要设置名为***FULL_TIME*** 的Secret。

##### 同步步数生成方式
    if 当前毫秒数 < FULL_TIME毫秒数 {
        同步步数 = 当前毫秒数/FULL_TIME毫秒数 * MAX_STEPS
    }else{
        同步步数 = MAX_STEPS
    }

#### 五、同步时间
同步时间决定同步次数及同步的步数，如果有需要请修改actions中的相关内容。

## 注意事项
- 同步不成功请确认是否关联账号。
- 邮箱同步未测试，理论可行！！！！！



