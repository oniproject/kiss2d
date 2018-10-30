// Package fixed implements fixed-point integer types.

pub trait AnyNum:
    Copy + Clone + Default + PartialEq + PartialOrd +
    std::ops::Add<Output=Self> +
    std::ops::Sub<Output=Self>
{
    fn floor(&self) -> isize;
    fn round(&self) -> isize;
    fn ceil(&self) -> isize;
    fn mul(self, other: Self) -> Self;
}

pub trait AnyPoint: Copy + Clone + Default + PartialEq +
    std::ops::Add<Output=Self> +
    std::ops::Sub<Output=Self>
{
    type Num: AnyNum;

    fn x(&self) -> Self::Num;
    fn y(&self) -> Self::Num;

    fn mul(self, rhs: Self::Num) -> Self;
    fn div(self, rhs: Self::Num) -> Self;
}

pub trait AnyRect: Copy + Clone + Default + PartialEq {
    type Num: AnyNum;
    type Point: AnyPoint<Num=Self::Num>;

    fn min(&self) -> Self::Point;
    fn max(&self) -> Self::Point;

    fn add(self, rhs: Self::Point) -> Self;
    fn sub(self, rhs: Self::Point) -> Self;

    #[inline(always)] fn dx(&self) -> Self::Num { self.max().x() - self.min().x() }
    #[inline(always)] fn dy(&self) -> Self::Num { self.max().y() - self.min().y() }

    #[inline(always)]
    /// Returns whether the rectangle contains no points.
    fn is_empty(&self) -> bool
        where Self::Num: Ord
    {
        self.min().x() >= self.max().x() || self.min().y() >= self.max().y()
    }

    #[inline(always)]
    fn contains_point(&self, p: &Self::Point) -> bool
        where Self::Num: Ord
    {
        self.min().x() <= p.x() && p.x() < self.max().x() &&
        self.min().y() <= p.y() && p.y() < self.max().y()
    }

    #[inline(always)]
    fn contains_rect(&self, other: &Self) -> bool
        where Self::Num: Ord
    {
        if self.is_empty() {
            true
        } else {
            // NOTE that r.Max is an exclusive bound for r, so that r.In(s)
            // does not require that r.Max.In(s).
            other.min().x() <= self.min().x() && self.max().x() <= other.max().x() &&
            other.min().y() <= self.min().y() && self.max().y() <= other.max().y()
        }
    }

    fn maybe_intersect(self, other: Self) -> Option<Self>;
    fn intersect(self, other: Self) -> Self;
    fn union(self, other: Self) -> Self;
}

#[doc(hidden)]
macro impl_fixed($name:ident, $point:ident, $rect:ident, $inner:ident, $outer:ty, $shift:expr) {
    /// Fixed-point number.
    #[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
    pub struct $name(pub $inner);

    impl $name {
        /// Returns the greatest integer value less than or equal to self.
        #[inline(always)]
        pub fn floor(&self) -> isize { (self.0 >> $shift) as isize }
        /// Returns the nearest integer value to self. Ties are rounded up.
        #[inline(always)]
        pub fn round(&self) -> isize {
            ($inner::wrapping_add(self.0, (1 << $shift) - 1) >> $shift) as isize
        }
        /// Returns the least integer value greater than or equal to self.
        #[inline(always)]
        pub fn ceil(&self) -> isize  {
            ($inner::wrapping_add(self.0, (1 << $shift)).wrapping_sub(1) >> $shift) as isize
        }

        #[inline(always)]
        pub fn mul(self, other: Self) -> Self {
            let x = self.0 as $outer;
            let y = other.0 as $outer;
            let value = (x * y + 1<<($shift-1)) >> $shift;
            $name(value as $inner)
        }
    }

    impl From<$inner> for $name {
        #[inline(always)]
        fn from(i: $inner) -> Self { $name(i << $shift) }
    }

    impl std::ops::Add for $name {
        type Output = Self;
        #[inline(always)]
        fn add(self, other: Self) -> Self { $name(self.0 + other.0) }
    }
    impl std::ops::Sub for $name {
        type Output = Self;
        #[inline(always)]
        fn sub(self, other: Self) -> Self { $name(self.0 - other.0) }
    }

    /// Fixed-point coordinate pair.
    #[derive(Copy, Clone, Default, PartialEq, Eq)]
    pub struct $point {
        pub x: $name,
        pub y: $name,
    }

    impl $point {
        /// Returns whether p is in r.
        #[inline(always)]
        pub fn in_rect(&self, r: &$rect) -> bool {
            r.min.x <= self.x && self.x < r.max.x &&
            r.min.y <= self.y && self.y < r.max.y
        }
    }

    impl std::ops::Add for $point {
        type Output = Self;
        #[inline(always)]
        fn add(self, other: Self) -> Self {
            Self {
                x: self.x + other.x,
                y: self.y + other.y,
            }
        }
    }

    impl std::ops::Sub for $point {
        type Output = Self;
        #[inline(always)]
        fn sub(self, other: Self) -> Self {
            Self {
                x: self.x - other.x,
                y: self.y - other.y,
            }
        }
    }

    impl std::ops::Mul<$name> for $point {
        type Output = Self;
        #[inline(always)]
        fn mul(self, k: $name) -> Self {
            Self {
                x: $name(self.x.0 * k.0 / (1 << $shift)),
                y: $name(self.y.0 * k.0 / (1 << $shift)),
            }
        }
    }

    impl std::ops::Div<$name> for $point {
        type Output = Self;
        #[inline(always)]
        fn div(self, k: $name) -> Self {
            Self {
                x: $name(self.x.0 * (1 << $shift) / k.0),
                y: $name(self.y.0 * (1 << $shift) / k.0),
            }
        }
    }

    /// Fixed-point coordinate rectangle.
    /// The Min bound is inclusive and the Max bound is exclusive.
    /// It is well-formed if Min.X <= Max.X and likewise for Y.
    #[derive(Copy, Clone, Default, PartialEq, Eq)]
    pub struct $rect {
        pub min: $point,
        pub max: $point,
    }

    impl std::ops::Add<$point> for $rect {
        type Output = Self;
        #[inline(always)]
        fn add(self, p: $point) -> Self {
            Self {
                min: self.min + p,
                max: self.max + p,
            }
        }
    }

    impl std::ops::Sub<$point> for $rect {
        type Output = Self;
        #[inline(always)]
        fn sub(self, p: $point) -> Self {
            Self {
                min: self.min - p,
                max: self.max - p,
            }
        }
    }

    impl $rect {
        /// Returns whether the rectangle contains no points.
        #[inline(always)]
        pub fn is_empty(&self) -> bool {
            self.min.x >= self.max.x || self.min.y >= self.max.y
        }

        /// Returns the largest rectangle contained by both r and s.
        /// If the two rectangles do not overlap then the zero rectangle will be returned.
        #[inline]
        pub fn intersect(self, other: Self) -> Self {
            let mut r = self;
            let s = other;
            if r.min.x < s.min.x { r.min.x = s.min.x }
            if r.min.y < s.min.y { r.min.y = s.min.y }
            if r.max.x > s.max.x { r.max.x = s.max.x }
            if r.max.y > s.max.y { r.max.y = s.max.y }
            if r.is_empty() { Self::default() } else { r }
        }

        /// Returns the largest rectangle contained by both r and s.
        /// If the two rectangles do not overlap then the zero rectangle will be returned.
        #[inline]
        pub fn maybe_intersect(self, other: Self) -> Option<Self> {
            let mut r = self;
            let s = other;
            if r.min.x < s.min.x { r.min.x = s.min.x }
            if r.min.y < s.min.y { r.min.y = s.min.y }
            if r.max.x > s.max.x { r.max.x = s.max.x }
            if r.max.y > s.max.y { r.max.y = s.max.y }
            if r.is_empty() { None } else { Some(r) }
        }

        /// Returns the smallest rectangle that contains both r and s.
        #[inline]
        pub fn union(self, other: Self) -> Self {
            let mut r = self;
            let s = other;
            if r.is_empty() { return s }
            if s.is_empty() { return r }
            if r.min.x > s.min.x { r.min.x = s.min.x }
            if r.min.y > s.min.y { r.min.y = s.min.y }
            if r.max.x < s.max.x { r.max.x = s.max.x }
            if r.max.y < s.max.y { r.max.y = s.max.y }
            r
        }

        /// Returns whether every point in r is in s.
        #[inline]
        pub fn in_rect(&self, other: &Self) -> bool {
            if self.is_empty() {
                true
            } else {
                // NOTE that r.Max is an exclusive bound for r, so that r.In(s)
                // does not require that r.Max.In(s).
                other.min.x <= self.min.x && self.max.x <= other.max.x &&
                other.min.y <= self.min.y && self.max.y <= other.max.y
            }
        }
    }
}

impl_fixed!(I26_6 , P26_6 , R26_6 , i32, i64, 6);
impl_fixed!(I52_12, P52_12, R52_12, i64, i128, 12);
