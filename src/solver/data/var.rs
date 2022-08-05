#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Var(u32);

impl Var {
    pub fn new(i: i32) -> Self {
        assert!(i > 0);
        let i = i as u32;

        assert_eq!(i & (0b11 << 30), 0);

        Var(i
            .try_into()
            .expect("Assert above checks that value is non-zero"))
    }

    pub fn get(self) -> u32 {
        self.0
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

// Literals are represented as u32.
// The LSB is one, iff the literal is negative.
// The MSB is *always* zero.
// The remaining bits represent the variable.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lit(u32);

impl Lit {
    pub fn new(lit: i32) -> Self {
        assert_ne!(lit, 0, "Literals cant be zero");

        let new_lit_repr = lit.abs() as u32;
        let new_lit_repr = (new_lit_repr << 1) | ((lit < 0) as u32);
        assert!(new_lit_repr & (1 << 31) == 0, "Lit magnitude too large.");

        Lit(new_lit_repr)
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.0 & (1 << 31) == 0
    }

    pub(crate) fn lit_slice_to_u32_slice(lits: &[Lit]) -> &[u32] {
        debug_assert!(lits.iter().all(Lit::is_valid));
        unsafe { std::mem::transmute(lits) }
    }

    pub fn var(self) -> Var {
        // Because we prevent the value of ever being `i32::MIN` the call to `abs` is always positive.
        // (i32::MIN.abs() < 0 due to twos complement shenanigans)
        Var(self.0 >> 1)
    }

    pub fn get(self) -> u32 {
        self.0
    }

    pub fn is_pos(self) -> bool {
        self.0 & 1 == 0
    }

    #[allow(unused)]
    pub fn is_neg(self) -> bool {
        self.0 & 1 == 1
    }
}

impl From<Var> for Lit {
    fn from(v: Var) -> Self {
        Lit(v.0 << 1)
    }
}

impl std::ops::Neg for Lit {
    type Output = Lit;

    fn neg(self) -> Self::Output {
        Lit(self.0 ^ 1)
    }
}

impl std::fmt::Debug for Lit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Lit")
            .field(&format!(
                "{}{}",
                if self.is_pos() { "" } else { "-" },
                self.var().get()
            ))
            .finish()
    }
}

impl std::fmt::Display for Lit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format!(
            "{}{}",
            if self.is_pos() { "" } else { "-" },
            self.var().get()
        )
        .fmt(f)
    }
}
