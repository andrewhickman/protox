#[derive()]
enum Component<'a> {
    Unescaped(&'a str),
    Terminator(char),
    Char(char),
    Error,
}
impl<'s> ::logos::Logos<'s> for Component<'s> {
    type Extras = ();
    type Source = str;
    const ERROR: Self = Component::Error;
    fn lex(lex: &mut ::logos::Lexer<'s, Self>) {
        use logos::internal::{CallbackResult, LexerInternal};
        type Lexer<'s> = ::logos::Lexer<'s, Component<'s>>;
        fn _end<'s>(lex: &mut Lexer<'s>) {
            lex.end()
        }
        fn _error<'s>(lex: &mut Lexer<'s>) {
            lex.bump_unchecked(1);
            lex.error();
        }
        static COMPACT_TABLE_0: [u8; 256] = [
            0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        ];
        macro_rules ! _fast_loop { ($ lex : ident , $ test : ident , $ miss : expr) => { while let Some (arr) = $ lex . read :: < & [u8 ; 16] > () { if $ test (arr [0]) { if $ test (arr [1]) { if $ test (arr [2]) { if $ test (arr [3]) { if $ test (arr [4]) { if $ test (arr [5]) { if $ test (arr [6]) { if $ test (arr [7]) { if $ test (arr [8]) { if $ test (arr [9]) { if $ test (arr [10]) { if $ test (arr [11]) { if $ test (arr [12]) { if $ test (arr [13]) { if $ test (arr [14]) { if $ test (arr [15]) { $ lex . bump_unchecked (16) ; continue ; } $ lex . bump_unchecked (15) ; return $ miss ; } $ lex . bump_unchecked (14) ; return $ miss ; } $ lex . bump_unchecked (13) ; return $ miss ; } $ lex . bump_unchecked (12) ; return $ miss ; } $ lex . bump_unchecked (11) ; return $ miss ; } $ lex . bump_unchecked (10) ; return $ miss ; } $ lex . bump_unchecked (9) ; return $ miss ; } $ lex . bump_unchecked (8) ; return $ miss ; } $ lex . bump_unchecked (7) ; return $ miss ; } $ lex . bump_unchecked (6) ; return $ miss ; } $ lex . bump_unchecked (5) ; return $ miss ; } $ lex . bump_unchecked (4) ; return $ miss ; } $ lex . bump_unchecked (3) ; return $ miss ; } $ lex . bump_unchecked (2) ; return $ miss ; } $ lex . bump_unchecked (1) ; return $ miss ; } return $ miss ; } while $ lex . test ($ test) { $ lex . bump_unchecked (1) ; } $ miss } ; }
        #[inline]
        fn goto4_x<'s>(lex: &mut Lexer<'s>) {
            terminator(lex).construct(Component::Terminator, lex);
        }
        #[inline]
        fn goto13_x<'s>(lex: &mut Lexer<'s>) {
            char_escape(lex).construct(Component::Char, lex);
        }
        #[inline]
        fn goto6_x<'s>(lex: &mut Lexer<'s>) {
            hex_escape(lex).construct(Component::Char, lex);
        }
        #[inline]
        fn pattern0(byte: u8) -> bool {
            const LUT: u64 = 35465847073801215u64;
            match 1u64.checked_shl(byte.wrapping_sub(48u8) as u32) {
                Some(shift) => LUT & shift != 0,
                None => false,
            }
        }
        #[inline]
        fn goto7_at3<'s>(lex: &mut Lexer<'s>) {
            let byte = match lex.read_at::<u8>(3usize) {
                Some(byte) => byte,
                None => return _error(lex),
            };
            match byte {
                byte if pattern0(byte) => {
                    lex.bump_unchecked(4usize);
                    goto6_x(lex)
                }
                _ => _error(lex),
            }
        }
        #[inline]
        fn goto8_at2<'s>(lex: &mut Lexer<'s>) {
            let arr = match lex.read_at::<&[u8; 2usize]>(2usize) {
                Some(arr) => arr,
                None => return _error(lex),
            };
            match arr[0] {
                byte if pattern0(byte) => goto7_at3(lex),
                _ => _error(lex),
            }
        }
        #[inline]
        fn goto11_x<'s>(lex: &mut Lexer<'s>) {
            oct_escape(lex).construct(Component::Char, lex);
        }
        #[inline]
        fn goto18_at2<'s>(lex: &mut Lexer<'s>) {
            match lex.read_at::<&[u8; 2usize]>(2usize) {
                Some([b'0'..=b'7', b'0'..=b'7']) => {
                    lex.bump_unchecked(4usize);
                    goto11_x(lex)
                }
                _ => _error(lex),
            }
        }
        #[inline]
        fn goto19_at1<'s>(lex: &mut Lexer<'s>) {
            enum Jump {
                __,
                J13,
                J8,
                J18,
            }
            const LUT: [Jump; 256] = {
                use Jump::*;
                [
                    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __,
                    __, __, __, __, __, __, __, __, __, __, __, __, __, __, J13, __, __, __, __,
                    J13, __, __, __, __, __, __, __, __, J18, J18, J18, J18, J18, J18, J18, J18,
                    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __,
                    __, __, __, __, __, __, __, __, __, __, __, __, J8, __, __, __, J13, __, __,
                    __, __, J13, J13, __, __, __, J13, __, __, __, __, __, __, __, J13, __, __, __,
                    J13, __, J13, __, J13, __, J8, __, __, __, __, __, __, __, __, __, __, __, __,
                    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __,
                    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __,
                    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __,
                    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __,
                    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __,
                    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __,
                    __, __, __,
                ]
            };
            let byte = match lex.read_at::<u8>(1usize) {
                Some(byte) => byte,
                None => return _error(lex),
            };
            match LUT[byte as usize] {
                Jump::J13 => {
                    lex.bump_unchecked(2usize);
                    goto13_x(lex)
                }
                Jump::J8 => goto8_at2(lex),
                Jump::J18 => goto18_at2(lex),
                Jump::__ => _error(lex),
            }
        }
        #[inline]
        fn goto1_ctx1_x<'s>(lex: &mut Lexer<'s>) {
            let token = Component::Unescaped(lex.slice());
            lex.set(token);
        }
        #[inline]
        fn pattern1(byte: u8) -> bool {
            COMPACT_TABLE_0[byte as usize] & 1 > 0
        }
        #[inline]
        fn goto2_ctx1_x<'s>(lex: &mut Lexer<'s>) {
            _fast_loop!(lex, pattern1, goto1_ctx1_x(lex));
        }
        #[inline]
        fn goto20<'s>(lex: &mut Lexer<'s>) {
            enum Jump {
                __,
                J4,
                J19,
                J2,
            }
            const LUT: [Jump; 256] = {
                use Jump::*;
                [
                    __, J2, J2, J2, J2, J2, J2, J2, J2, J2, __, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J4, J2, J2, J2, J2, J4,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J19, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                    J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2, J2,
                ]
            };
            let byte = match lex.read::<u8>() {
                Some(byte) => byte,
                None => return _end(lex),
            };
            match LUT[byte as usize] {
                Jump::J4 => {
                    lex.bump_unchecked(1usize);
                    goto4_x(lex)
                }
                Jump::J19 => goto19_at1(lex),
                Jump::J2 => {
                    lex.bump_unchecked(1usize);
                    goto2_ctx1_x(lex)
                }
                Jump::__ => _error(lex),
            }
        }
        goto20(lex)
    }
}
