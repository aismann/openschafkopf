pub fn assign_better<T>(dst: &mut T, src: T, fn_better: impl FnOnce(&T, &T) -> bool) -> bool {
    if fn_better(&src, dst) {
        *dst = src;
        true
    } else {
        false
    }
}

pub fn assign_other<T: Eq>(dst: &mut T, src: T) -> bool {
    assign_better(dst, src, |lhs, rhs| lhs!=rhs)
}

pub fn assign_min<T: Ord>(dst: &mut T, src: T) -> bool {
    assign_better(dst, src, |lhs, rhs| lhs<rhs)
}

pub fn assign_max<T: Ord>(dst: &mut T, src: T) -> bool {
    assign_better(dst, src, |lhs, rhs| lhs>rhs)
}

pub fn assign_by_key_ordering<T, K: Ord, FnKey>(
    dst: &mut T,
    src: T,
    mut fn_key: FnKey,
    ordering: std::cmp::Ordering,
) -> bool where
    FnKey: FnMut(&T) -> K,
{
    assign_better(dst, src, |lhs, rhs| {
        ordering == fn_key(lhs).cmp(&fn_key(rhs))
    })
}

#[test]
fn test_assign_by_key_ordering() {
    let mut n = 0;
    let b = assign_by_key_ordering(&mut n, 1, |t| *t, std::cmp::Ordering::Greater);
    assert!(b);
    assert_eq!(n, 1);
    let b = assign_by_key_ordering(&mut n, 0, |t| *t, std::cmp::Ordering::Less);
    assert!(b);
    assert_eq!(n, 0);
}
