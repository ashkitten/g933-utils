#[macro_export]
macro_rules! v {
    [ #PUSH $v:ident; @$arr:expr $(,)* ] => {
        $v.extend($arr);
    };
    [ #PUSH $v:ident; @$arr:expr, $($rest:tt)* ] => {
        $v.extend($arr);
        v![ #PUSH $v; $($rest)* ];
    };
    [ #PUSH $v:ident; $val:expr $(,)* ] => {
        $v.push($val);
    };
    [ #PUSH $v:ident; $val:expr, $($rest:tt)* ] => {
        $v.push($val);
        v![ #PUSH $v; $($rest)* ];
    };
    // Init
    [ $($rest:tt)* ] => {
        {
            let mut v = Vec::new();
            v![ #PUSH v; $($rest)*];
            v
        }
    }
}
