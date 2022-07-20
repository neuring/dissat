use std::num::NonZeroI32;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Var(NonZeroI32);

impl Var {
    pub fn new(i: i32) -> Self {
        assert!(i > 0);

        Var(i
            .try_into()
            .expect("Assert above checks that value is non-zero"))
    }

    pub fn get(self) -> i32 {
        self.0.get()
    }
}

impl std::fmt::Debug for Var {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Display for Var {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lit(NonZeroI32);

impl Lit {
    pub fn new(i: i32) -> Self {
        assert!(i > i32::MIN); // To ensure negation is well behaved, we disallow `i32::MIN`
                               // because -i32::MIN = i32::MIN
        Lit(i.try_into().expect("value of lit cannot be zero"))
    }

    pub fn var(self) -> Var {
        // Because we prevent the value of ever being `i32::MIN` the call to `abs` is always positive.
        // (i32::MIN.abs() < 0 due to twos complement shenanigans)
        Var(self.0.abs())
    }

    pub fn get(self) -> i32 {
        self.0.get()
    }

    pub fn is_pos(self) -> bool {
        self.get() > 0
    }

    #[allow(unused)]
    pub fn is_neg(self) -> bool {
        self.get() < 0
    }
}

impl From<Var> for Lit {
    fn from(v: Var) -> Self {
        Lit(v.0)
    }
}

impl std::ops::Neg for Lit {
    type Output = Lit;

    fn neg(self) -> Self::Output {
        // Safety: `Lit::new` ensures that its value is not zero.
        //          Therefore, if we negate its value it has to be non-zero as well.
        unsafe { Lit(NonZeroI32::new_unchecked(-self.0.get())) }
    }
}

impl std::fmt::Debug for Lit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Display for Lit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
