//! Independent dense-state acceptance tests. No simulator arithmetic is reused.
use ruqu_independent_stabilizer::{Gate, Tableau};

#[derive(Clone, Copy, Debug, Default)]
struct C { re: f64, im: f64 }
impl C {
    fn add(self, b: Self) -> Self { Self { re: self.re + b.re, im: self.im + b.im } }
    fn sub(self, b: Self) -> Self { Self { re: self.re - b.re, im: self.im - b.im } }
    fn scale(self, s: f64) -> Self { Self { re: self.re * s, im: self.im * s } }
    fn i(self) -> Self { Self { re: -self.im, im: self.re } }
    fn norm2(self) -> f64 { self.re*self.re + self.im*self.im }
}
struct Oracle { a: Vec<C> }
impl Oracle {
    fn new(n: usize) -> Self { let mut a = vec![C::default(); 1 << n]; a[0].re = 1.; Self { a } }
    fn h(&mut self, q: usize) {
        for b in 0..self.a.len() { if b & (1 << q) == 0 {
            let c = b | (1 << q); let (u,v) = (self.a[b],self.a[c]);
            self.a[b] = u.add(v).scale(std::f64::consts::FRAC_1_SQRT_2);
            self.a[c] = u.sub(v).scale(std::f64::consts::FRAC_1_SQRT_2);
        }}
    }
    fn s(&mut self, q: usize) { for b in 0..self.a.len() { if b & (1<<q) != 0 { self.a[b] = self.a[b].i(); } } }
    fn x(&mut self, q: usize) { for b in 0..self.a.len() { if b & (1<<q) == 0 { self.a.swap(b,b|(1<<q)); } } }
    fn z(&mut self, q: usize) { for b in 0..self.a.len() { if b & (1<<q) != 0 { self.a[b] = self.a[b].scale(-1.); } } }
    fn y(&mut self, q: usize) { self.z(q); self.x(q); for a in &mut self.a { *a = a.i(); } }
    fn cx(&mut self, c: usize, t: usize) { for b in 0..self.a.len() { if b&(1<<c)!=0 && b&(1<<t)==0 { self.a.swap(b,b|(1<<t)); } } }
    fn measure(&mut self, q: usize, random: bool) -> bool {
        let p: f64 = self.a.iter().enumerate().filter(|(b,_)| b&(1<<q)!=0).map(|(_,a)|a.norm2()).sum();
        let result = if p < 1e-12 { false } else if 1.-p < 1e-12 { true } else { assert!((p-0.5).abs()<1e-12); random };
        let weight = if result { p } else { 1.-p };
        for (b,a) in self.a.iter_mut().enumerate() { *a = if (b&(1<<q)!=0)==result { a.scale(weight.sqrt().recip()) } else { C::default() }; }
        result
    }
}
fn bit(words: &[u32], q: usize) -> bool { words[q/32] & (1 << (q%32)) != 0 }
fn check_structure(t: &Tableau) {
    let n=t.qubits(); let words=(n+31)/32;
    for i in 0..2*n {
        let a=t.row(i).expect("missing row"); assert_eq!(a.x.len(),words); assert_eq!(a.z.len(),words);
        if n%32!=0 { let mask=!((1u32<<(n%32))-1); assert_eq!(a.x[words-1]&mask,0); assert_eq!(a.z[words-1]&mask,0); }
        for j in i..2*n {
            let b=t.row(j).unwrap(); let parity=a.x.iter().zip(b.z).zip(a.z.iter().zip(b.x)).fold(0u32,|s,((x,z),(u,v))|s^((x&z)^(u&v)).count_ones())&1;
            assert_eq!(parity==1, i<n && j==n+i,"symplectic rows {i},{j}");
        }
    }
    assert!(t.row(2*n).is_none());
}
fn check_dense(t: &Tableau, o: &Oracle) {
    check_structure(t); let n=t.qubits();
    assert!((o.a.iter().map(|a|a.norm2()).sum::<f64>()-1.).abs()<1e-10);
    for r in n..2*n {
        let row=t.row(r).unwrap(); let mut x=0usize; let mut z=0usize; let mut p=if row.negative {2} else {0};
        for q in 0..n { if bit(row.x,q) { x|=1<<q; } if bit(row.z,q) { z|=1<<q; } if bit(row.x,q)&&bit(row.z,q) {p+=1;} }
        let mut error=0.;
        for b in 0..o.a.len() { let mut a=o.a[b]; for _ in 0..p%4 {a=a.i();} if (z&b).count_ones()%2!=0 {a=a.scale(-1.);} error+=a.sub(o.a[b^x]).norm2(); }
        assert!(error.sqrt()<1e-10,"signed stabilizer row {r}, residual {}",error.sqrt());
    }
}
fn next(s: &mut u64) -> u64 { *s=s.wrapping_add(0x9e3779b97f4a7c15); let mut z=*s; z=(z^(z>>30)).wrapping_mul(0xbf58476d1ce4e5b9); z=(z^(z>>27)).wrapping_mul(0x94d049bb133111eb); z^(z>>31) }
fn seeded(seed: u64) {
    let mut rng=seed;
    for n in [1,2,3,5,8] {
        let mut t=Tableau::new(n).unwrap(); let mut o=Oracle::new(n); check_dense(&t,&o);
        for step in 0..96 {
            let q=next(&mut rng) as usize%n; let op=next(&mut rng)%9;
            match op {
                0=>{t.apply(Gate::H(q)).unwrap();o.h(q)},
                1=>{t.apply(Gate::S(q)).unwrap();o.s(q)},
                2=>{t.apply(Gate::X(q)).unwrap();o.x(q)},
                3=>{t.apply(Gate::Y(q)).unwrap();o.y(q)},
                4=>{t.apply(Gate::Z(q)).unwrap();o.z(q)},
                5 if n>1=>{let c=(q+1+next(&mut rng) as usize%(n-1))%n;t.apply(Gate::Cx(c,q)).unwrap();o.cx(c,q)},
                7=>{let random=next(&mut rng)&1!=0;t.apply(Gate::Reset(q,random)).unwrap();if o.measure(q,random) {o.x(q);}},
                _=>{let random=next(&mut rng)&1!=0; let expected=o.measure(q,random); assert_eq!(t.measure_z(q,random).unwrap(),expected,"seed {seed}, step {step}"); assert_eq!(t.measure_z(q,!random).unwrap(),expected,"repeat measurement");}
            }
            check_dense(&t,&o);
        }
    }
}
#[test]
fn thirty_development_seeds_dense_oracle() { for seed in 0..30 {seeded(seed);} }
#[test]
fn ten_separate_confirmation_seeds_dense_oracle() {
    // Evaluator-owned confirmation corpus, separate from the development seed set.
    for seed in [0x91483ace7031b952,0x73b428e905fd176c,0xd761890cabb4f302,0x328fa671dd095eb4,0xb724ff008135a9ce,0x58db40931e2a77c6,0xeff20a795408cbd1,0x403b98cdd7642ae5,0xaac037149d268be2,0x12f50eab69c4873d] {seeded(seed);}
}
#[test]
fn adversarial_phase_and_bell_measurements() {
    for random in [false,true] {
        let mut t=Tableau::new(2).unwrap();let mut o=Oracle::new(2);
        for _ in 0..9 {
            t.apply(Gate::H(0)).unwrap();o.h(0);check_dense(&t,&o);
            t.apply(Gate::S(0)).unwrap();o.s(0);check_dense(&t,&o);
            t.apply(Gate::Cx(0,1)).unwrap();o.cx(0,1);check_dense(&t,&o);
            t.apply(Gate::S(1)).unwrap();o.s(1);check_dense(&t,&o);
        }
        for q in [1,0,1,0] {assert_eq!(t.measure_z(q,random).unwrap(),o.measure(q,random));check_dense(&t,&o);}
        let mut t=Tableau::new(2).unwrap();t.apply(Gate::H(0)).unwrap();t.apply(Gate::Cx(0,1)).unwrap();
        assert_eq!(t.measure_z(0,random).unwrap(),random);assert_eq!(t.measure_z(1,!random).unwrap(),random);
    }
}
#[test]
fn packed_word_boundaries_analytic_ghz() {
    for n in [31,32,33,255,256,257] { for random in [false,true] {
        let mut t=Tableau::new(n).unwrap();t.apply(Gate::H(0)).unwrap();for q in 1..n {t.apply(Gate::Cx(0,q)).unwrap();}check_structure(&t);
        // A signed Pauli stabilizes GHZ iff its X mask is all zero or all one,
        // its Z weight is even, and i^(2*sign + Y_count) equals +1.
        for r in n..2*n {let row=t.row(r).unwrap();let xcount=(0..n).filter(|&q|bit(row.x,q)).count();let zcount=(0..n).filter(|&q|bit(row.z,q)).count();let ycount=(0..n).filter(|&q|bit(row.x,q)&&bit(row.z,q)).count();assert!(xcount==0||xcount==n);assert_eq!(zcount%2,0);assert_eq!((2*usize::from(row.negative)+ycount)%4,0);}
        assert_eq!(t.measure_z(n-1,random).unwrap(),random);
        for q in 0..n {assert_eq!(t.measure_z(q,!random).unwrap(),random);}check_structure(&t);
        for r in n..2*n {let row=t.row(r).unwrap();assert!(row.x.iter().all(|&w|w==0));let parity=row.z.iter().map(|w|w.count_ones()).sum::<u32>()%2!=0;assert_eq!(row.negative,random&&parity);}
    }}
}
#[test]
fn invalid_gate_indices_are_rejected() {
    let mut t=Tableau::new(2).unwrap();
    for gate in [Gate::H(2),Gate::S(2),Gate::X(2),Gate::Y(2),Gate::Z(2),Gate::Reset(2,false),Gate::Cx(2,0),Gate::Cx(0,2),Gate::Cx(1,1)] { assert!(t.apply(gate).is_err()); }
    assert!(t.measure_z(2,false).is_err());check_dense(&t,&Oracle::new(2));
}
