use bevy_stat_query::Fraction;

#[test]
pub fn reduction() {
    macro_rules! test_reduction {
        ($a: expr, $b: expr) => {{
            let v = Fraction::new($a, $b);
            assert_eq!(v, Fraction::new_raw($a, $b));
            let v = Fraction::<i32>::const_new($a, $b);
            assert_eq!(v, Fraction::new_raw($a, $b));
            let v = Fraction::new_raw($a, $b);
            assert_eq!(v.reduced(), Fraction::new_raw($a, $b));
        }};
        ($a: expr, $b: expr, $c: expr, $d: expr) => {{
            let v = Fraction::new($a, $b);
            assert_eq!(v, Fraction::new_raw($c, $d));
            let v = Fraction::<i32>::const_new($a, $b);
            assert_eq!(v, Fraction::new_raw($c, $d));
            let v = Fraction::new_raw($a, $b);
            assert_eq!(v.reduced(), Fraction::new_raw($c, $d));
        }};
    }
    test_reduction!(1, 1);
    test_reduction!(0, 1);
    test_reduction!(1, 0);
    test_reduction!(2, 1);
    test_reduction!(4, 1);
    test_reduction!(1, 3);
    test_reduction!(2, 3);
    test_reduction!(-2, 3);
    test_reduction!(-3, 2);
    test_reduction!(-2, -3);
    test_reduction!(-5, -3);
    test_reduction!(2, 4, 1, 2);
    test_reduction!(4, 6, 2, 3);
    test_reduction!(15, 10, 3, 2);
    test_reduction!(-25, 25, -1, 1);
    test_reduction!(-40, -60, -2, -3);
}

#[test]
pub fn rounding() {
    fn f(a: i32, b: i32) -> Fraction<i32> {
        Fraction::new_raw(a, b)
    }

    assert_eq!(f(0, 1).floor(), 0);
    assert_eq!(f(0, 1).ceil(), 0);
    assert_eq!(f(0, 1).round(), 0);
    assert_eq!(f(0, 1).trunc(), 0);

    assert_eq!(f(1, 1).floor(), 1);
    assert_eq!(f(1, 1).ceil(), 1);
    assert_eq!(f(1, 1).round(), 1);
    assert_eq!(f(1, 1).trunc(), 1);

    assert_eq!(f(1, 2).floor(), 0);
    assert_eq!(f(1, 2).ceil(), 1);
    assert_eq!(f(1, 2).round(), 1);
    assert_eq!(f(1, 2).trunc(), 0);

    assert_eq!(f(5, 2).floor(), 2);
    assert_eq!(f(5, 2).ceil(), 3);
    assert_eq!(f(5, 2).round(), 3);
    assert_eq!(f(5, 2).trunc(), 2);

    assert_eq!(f(-1, 1).floor(), -1);
    assert_eq!(f(-1, 1).ceil(), -1);
    assert_eq!(f(-1, 1).round(), -1);
    assert_eq!(f(-1, 1).trunc(), -1);

    assert_eq!(f(1, -2).floor(), -1);
    assert_eq!(f(1, -2).ceil(), 0);
    assert_eq!(f(1, -2).round(), -1);
    assert_eq!(f(1, -2).trunc(), 0);

    assert_eq!(f(7, 3).floor(), 2);
    assert_eq!(f(7, 3).ceil(), 3);
    assert_eq!(f(7, 3).trunc(), 2);
    assert_eq!(f(7, 3).round(), 2);
    assert_eq!(f(8, 3).round(), 3);

    assert_eq!(f(7, -3).floor(), -3);
    assert_eq!(f(7, -3).ceil(), -2);
    assert_eq!(f(7, -3).trunc(), -2);
    assert_eq!(f(7, -3).trunc(), -2);
    assert_eq!(f(8, -3).round(), -3);
}
