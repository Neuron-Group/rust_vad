pub struct Synaptic {
    v_t: f32,
    k_p: f32,
    t_max: f32,
    a_r: f32,
    a_d: f32,
    dt: f32,
    s: f32,
}

impl Synaptic {
    pub fn new() -> Self {
        Self {
            v_t: 0.5,   // 阈值电位
            k_p: 0.02,  // 激活曲线陡度
            t_max: 1.0, // 最大递质释放量
            a_r: 600.0, // 上升速率
            a_d: 100.0, // 衰减速率
            dt: 0.001,  // 时间步长
            s: 0.0,     // 递质浓度初始值
        }
    }

    pub fn init(&mut self) {
        self.s = 0.0;
    }

    pub fn update(&mut self, v_pre: f32) -> f32 {
        // 计算神经递质浓度
        let release_prob = self.t_max / (1.0 + (-(v_pre - self.v_t) / self.k_p).exp());

        // 定义微分方程 ds/dt = f(s)
        let f = |s: f32| self.a_r * release_prob * (1.0 - s) - self.a_d * s;

        // 四阶龙格-库塔方法
        let k1 = f(self.s);
        let k2 = f(self.s + 0.5 * self.dt * k1);
        let k3 = f(self.s + 0.5 * self.dt * k2);
        let k4 = f(self.s + self.dt * k3);

        self.s += (self.dt / 6.0) * (k1 + 2.0 * k2 + 2.0 * k3 + k4);

        self.s = self.s.clamp(0.0, 1.0);

        self.s
    }
}

impl Default for Synaptic {
    fn default() -> Self {
        Self::new()
    }
}
