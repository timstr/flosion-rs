use std::default::Default;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Copy, Clone)]
pub struct Sample {
    pub l: f32,
    pub r: f32,
}

impl Sample {
    pub fn new(l: f32, r: f32) -> Sample {
        Sample { l, r }
    }

    pub fn silence(&mut self) {
        self.l = 0.0;
        self.r = 0.0;
    }
}

impl Default for Sample {
    fn default() -> Sample {
        Sample::new(0.0, 0.0)
    }
}

impl Add<Sample> for Sample {
    type Output = Sample;
    fn add(self, s: Sample) -> Sample {
        Sample {
            l: self.l + s.l,
            r: self.r + s.r,
        }
    }
}

impl AddAssign<Sample> for Sample {
    fn add_assign(&mut self, s: Sample) {
        self.l += s.l;
        self.r += s.r;
    }
}

impl Sub<Sample> for Sample {
    type Output = Sample;
    fn sub(self, s: Sample) -> Sample {
        Sample {
            l: self.l - s.l,
            r: self.r - s.r,
        }
    }
}

impl SubAssign<Sample> for Sample {
    fn sub_assign(&mut self, s: Sample) {
        self.l -= s.l;
        self.r -= s.r;
    }
}

impl Mul<f32> for Sample {
    type Output = Sample;
    fn mul(self, x: f32) -> Sample {
        Sample {
            l: self.l * x,
            r: self.r * x,
        }
    }
}

impl Mul<Sample> for f32 {
    type Output = Sample;
    fn mul(self, s: Sample) -> Sample {
        Sample {
            l: self * s.l,
            r: self * s.r,
        }
    }
}

impl MulAssign<f32> for Sample {
    fn mul_assign(&mut self, x: f32) {
        self.l *= x;
        self.r *= x;
    }
}

impl Div<f32> for Sample {
    type Output = Sample;
    fn div(self, x: f32) -> Sample {
        Sample {
            l: self.l / x,
            r: self.r / x,
        }
    }
}

impl Div<Sample> for f32 {
    type Output = Sample;
    fn div(self, s: Sample) -> Sample {
        Sample {
            l: s.l / self,
            r: s.r / self,
        }
    }
}

impl DivAssign<f32> for Sample {
    fn div_assign(&mut self, x: f32) {
        self.l /= x;
        self.r /= x;
    }
}

impl Neg for Sample {
    type Output = Sample;
    fn neg(self) -> Sample {
        Sample {
            l: -self.l,
            r: -self.r,
        }
    }
}
