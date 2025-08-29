use crate::type_trait::*;

pub struct Synaptic<Ft: FloatTrait> {
    v_t: Ft,
    k_p: Ft,
    t_max: Ft,
    a_r: Ft,
    a_d: Ft,
    dt: Ft,
    s: Ft,
}

impl<Ft> Synaptic<Ft>
where
    Ft: FloatTrait,
{
    pub fn new() -> Self {
        Self {
            v_t: Ft::from_f32(0.5).expect("Failed to convert v_t"), // 阈值电位
            k_p: Ft::from_f32(0.02).expect("Failed to convert k_p"), // 激活曲线陡度
            t_max: Ft::from_f32(1.0).expect("Failed to convert t_max"), // 最大递质释放量
            a_r: Ft::from_f32(600.0).expect("Failed to convert a_r"), // 上升速率
            a_d: Ft::from_f32(70.0).expect("Failed to convert a_d"), // 衰减速率
            dt: Ft::from_f32(0.001).expect("Failed to convert dt"), // 时间步长
            s: Ft::zero(),                                          // 初始递质浓度
        }
    }

    pub fn init(&mut self) {
        self.s = Ft::zero();
    }

    pub fn update(&mut self, v_pre: Ft) -> Ft {
        // 计算递质释放概率 (使用 Ft 的 exp 方法)
        let exp_arg = -((v_pre - self.v_t) / self.k_p);
        let release_prob = self.t_max / (Ft::one() + exp_arg.exp());

        // 定义微分方程 ds/dt = f(s)
        let f = |s: Ft| self.a_r * release_prob * (Ft::one() - s) - self.a_d * s;

        // 四阶龙格-库塔常量
        let two = Ft::from_f32(2.0).unwrap();
        let half = Ft::from_f32(0.5).unwrap();
        let sixth = Ft::from_f32(1.0 / 6.0).unwrap();

        // RK4 计算
        let k1 = f(self.s);
        let k2 = f(self.s + half * self.dt * k1);
        let k3 = f(self.s + half * self.dt * k2);
        let k4 = f(self.s + self.dt * k3);

        // 更新递质浓度 (使用 Ft 的运算)
        self.s = self.s + self.dt * sixth * (k1 + two * k2 + two * k3 + k4);

        // 限制在 [0,1] 区间
        self.s = self.s.max(Ft::zero()).min(Ft::one());

        self.s
    }
}

impl<Ft: FloatTrait> Default for Synaptic<Ft> {
    fn default() -> Self {
        Self::new()
    }
}
