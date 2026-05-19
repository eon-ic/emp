use std::time::{Duration, Instant};

pub struct Stopwatch {
    start_time: Option<Instant>, // 记录当前这一段计时的开始时间点
    elapsed_time: Duration,      // 记录过去已经累积的、已暂停好几段的时间总和
}

impl Stopwatch {
    // 初始化
    pub fn new() -> Self {
        Self {
            start_time: None,
            elapsed_time: Duration::ZERO,
        }
    }

    // 开始 / 恢复计时
    pub fn start(&mut self) {
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
        }
    }

    // 暂停计时
    pub fn pause(&mut self) {
        if let Some(start) = self.start_time.take() {
            // 把当前这一段经过的时间，累加到总时间里
            self.elapsed_time += start.elapsed();
        }
    } // 3. 核心新增：恢复计时（只有在“已启动过”且“当前处于暂停”时才有效）
    pub fn resume(&mut self) {
        if self.has_started() && self.start_time.is_none() {
            // 重新标记当前时间点作为新一段计时的起点
            self.start_time = Some(Instant::now());
        }
    }

    // 核心新增功能：判断当前是否正在计时（是否已启动且未暂停）
    pub fn is_running(&self) -> bool {
        self.start_time.is_some()
    } // 2. 核心新增功能：判断是否【已经开始】（即使处于暂停状态也算）
    pub fn has_started(&self) -> bool {
        // 正在运行，或者已经累加了大于 0 的时间值
        self.start_time.is_some() || self.elapsed_time > Duration::ZERO
    }
    // 获取当前总耗时（无论当前是运行还是暂停状态）
    pub fn elapsed(&self) -> Duration {
        match self.start_time {
            // 如果正在运行：已累积时间 + 当前这一段走过的时间
            Some(start) => self.elapsed_time + start.elapsed(),
            // 如果处于暂停：直接返回之前累积的时间
            None => self.elapsed_time,
        }
    }

    // 重置清零
    pub fn reset(&mut self) {
        self.start_time = None;
        self.elapsed_time = Duration::ZERO;
    }
    // 核心新增功能：设置/强转当前的计时值
    pub fn set_time(&mut self, new_time: Duration) {
        if self.start_time.is_some() {
            // 如果正在运行：将已累加时间设为新值，并把起点重置为“当下”
            self.elapsed_time = new_time;
            self.start_time = Some(Instant::now());
        } else {
            // 如果处于暂停：直接修改累加值即可
            self.elapsed_time = new_time;
        }
    }
}
