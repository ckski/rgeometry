use claim::debug_assert_ok;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::*;
use rand::distributions::{Distribution, Standard};
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::BTreeSet;
use std::ops::*;

use crate::array::Orientation;
use crate::data;
use crate::data::Point;
use crate::data::PointLocation;
use crate::data::Vector;
use crate::transformation::*;
use crate::{Error, PolygonScalar};

use super::Polygon;

#[derive(Debug, Clone)]
pub struct ConvexPolygon<T, P = ()>(Polygon<T, P>);

///////////////////////////////////////////////////////////////////////////////
// ConvexPolygon

impl<T, P> ConvexPolygon<T, P>
where
  T: PolygonScalar,
{
  /// O(1) Assume that a polygon is convex.
  ///
  /// # Safety
  /// The input polygon has to be strictly convex, ie. no vertices are allowed to
  /// be concave or colinear.
  pub unsafe fn new_unchecked(poly: Polygon<T, P>) -> ConvexPolygon<T, P> {
    let convex = ConvexPolygon(poly);
    debug_assert_ok!(convex.validate());
    convex
  }

  // O(log n)
  pub fn locate(&self, pt: &Point<T, 2>) -> PointLocation {
    debug_assert_ok!(self.validate());
    let poly = &self.0;
    let p0 = poly.vertex(0);
    let mut lower = 1;
    let mut upper = poly.points.len() as isize - 1;
    while lower + 1 < upper {
      let middle = (lower + upper) / 2;
      if p0.orientation(poly.vertex(middle), pt) == Orientation::CounterClockWise {
        lower = middle;
      } else {
        upper = middle;
      }
    }
    let p1 = poly.vertex(lower);
    let p2 = poly.vertex(upper);
    let triangle = data::TriangleView::new([p0, p1, p2]);
    triangle.locate(pt)
  }

  pub fn validate(&self) -> Result<(), Error> {
    let len = self.0.points.len() as isize;
    for i in 0..len {
      if self.0.vertex_orientation(i) != Orientation::CounterClockWise {
        return Err(Error::ConvexViolation);
      }
    }
    self.0.validate()
  }

  pub fn polygon(&self) -> &Polygon<T, P> {
    self.into()
  }
}

///////////////////////////////////////////////////////////////////////////////
// ConvexPolygon<BigRational>

impl ConvexPolygon<BigRational> {
  /// ```no_run
  /// # use rgeometry_wasm::playground::*;
  /// # use rgeometry::data::*;
  /// # let convex = {
  /// let mut rng = rand::thread_rng();
  /// ConvexPolygon::random(3, 1000, &mut rng)
  /// # };
  /// # render_polygon(convex.into());
  /// # return ()
  /// ```
  /// <iframe src="https://web.rgeometry.org:20443/loader.html?hash=36XCQBE0Yok="></iframe>
  pub fn random<R>(n: usize, max: usize, rng: &mut R) -> ConvexPolygon<BigRational>
  where
    R: Rng + ?Sized,
  {
    if n < 3 {
      // Return Result<P, Error> instead?
      return ConvexPolygon::random(3, max, rng);
    }
    let vs = {
      let mut vs = random_vectors(n, max, rng);
      Vector::sort_around(&mut vs);
      vs
    };
    let vertices: Vec<Point<BigRational, 2>> = vs
      .into_iter()
      .scan(Point::zero(), |st, vec| {
        *st += vec;
        Some(st.clone())
      })
      .collect();
    let n_vertices = (*vertices).len();
    debug_assert_eq!(n_vertices, n);
    let p = Polygon::new(vertices).unwrap();
    for i in 0..n {
      if p.vertex_orientation(i as isize) != Orientation::CounterClockWise {
        return Self::random(n, max, rng);
      }
    }
    let centroid = p.centroid();
    let t = Transform::translate(-Vector::from(centroid));
    let s = Transform::uniform_scale(BigRational::new(
      One::one(),
      BigInt::from_usize(max).unwrap(),
    ));
    ConvexPolygon(s * t * p)
  }
}

///////////////////////////////////////////////////////////////////////////////
// Trait Implementations

impl<T: PolygonScalar, P> Deref for ConvexPolygon<T, P> {
  type Target = Polygon<T, P>;
  fn deref(&self) -> &Self::Target {
    self.polygon()
  }
}

impl<T, P> From<ConvexPolygon<T, P>> for Polygon<T, P> {
  fn from(convex: ConvexPolygon<T, P>) -> Polygon<T, P> {
    convex.0
  }
}

impl<'a, T, P> From<&'a ConvexPolygon<T, P>> for &'a Polygon<T, P> {
  fn from(convex: &'a ConvexPolygon<T, P>) -> &'a Polygon<T, P> {
    &convex.0
  }
}

impl Distribution<ConvexPolygon<BigRational>> for Standard {
  fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ConvexPolygon<BigRational> {
    ConvexPolygon::random(100, usize::MAX, rng)
  }
}

///////////////////////////////////////////////////////////////////////////////
// Helper functions

// Property: random_between(n, max, &mut rng).iter().sum::<usize>() == max
fn random_between<R>(n: usize, max: usize, rng: &mut R) -> Vec<usize>
where
  R: Rng + ?Sized,
{
  debug_assert!(n > 0);
  assert!(n <= max);
  if max == n {
    return vec![1; n];
  }
  let mut pts = BTreeSet::new();
  while pts.len() < n - 1 {
    pts.insert(rng.gen_range(1..max));
  }
  let mut from = 0;
  let mut out = Vec::new();
  for x in pts.iter() {
    out.push(x - from);
    from = *x;
  }
  out.push(max - from);
  out
}

// Property: random_between_zero(10, 100, &mut rng).iter().sum::<isize>() == 0
// Property: random_between_zero(10, 100, &mut rng).iter().all(|v| !v.is_zero())
fn random_between_zero<R>(n: usize, max: usize, rng: &mut R) -> Vec<BigInt>
where
  R: Rng + ?Sized,
{
  assert!(n >= 2);
  let n_positive = rng.gen_range(1..n); // [1;n[
  dbg!(n_positive);
  let positive = random_between(n_positive, max, rng)
    .into_iter()
    .map(BigInt::from)
    .collect();
  let n_negative = n - n_positive;
  let negative: Vec<BigInt> = random_between(n_negative, max, rng)
    .into_iter()
    .map(BigInt::from)
    .map(Neg::neg)
    .collect();
  let mut result = [positive, negative].concat();
  result.shuffle(rng);
  result
  // random_between(n, max, rng)
  //   .into_iter()
  //   .map(BigInt::from)
  //   .zip(random_between(n, max, rng).into_iter().map(BigInt::from))
  //   .map(|(a, b)| a - b)
  //   .collect()
}

// Random vectors that sum to zero.
fn random_vectors<R>(n: usize, max: usize, rng: &mut R) -> Vec<Vector<BigRational, 2>>
where
  R: Rng + ?Sized,
{
  random_between_zero(n, max, rng)
    .into_iter()
    .zip(random_between_zero(n, max, rng).into_iter())
    .map(|(a, b)| Vector([BigRational::from(a), BigRational::from(b)]))
    .collect()
}

///////////////////////////////////////////////////////////////////////////////
// Tests

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Orientation::*;
  use crate::*;

  use proptest::prelude::*;
  use proptest::strategy::*;
  use proptest::test_runner::*;

  use ordered_float::NotNan;

  impl Arbitrary for ConvexPolygon<BigRational> {
    type Strategy = Just<ConvexPolygon<BigRational>>;
    type Parameters = ();
    fn arbitrary_with(_params: ()) -> Self::Strategy {
      Self::arbitrary()
    }
    fn arbitrary() -> Self::Strategy {
      let mut rng = rand::thread_rng();
      let n = rng.gen_range(3..=100);
      let max = rng.gen_range(n..=1_000_000_000);
      let p = ConvexPolygon::random(n, max, &mut rng);
      Just(p)
    }
  }

  proptest! {
    #[test]
    fn all_random_convex_polygons_are_valid(poly: ConvexPolygon<BigRational>) {
      prop_assert_eq!(poly.validate(), Ok(()))
    }

    #[test]
    fn sum_to_max(n in 1..1000, max in 0..1_000_000) {
      let mut rng = rand::thread_rng();
      let max = std::cmp::max(max, n);
      let vecs = random_between(n as usize, max as usize, &mut rng);
      prop_assert_eq!(vecs.iter().sum::<usize>(), max as usize)
    }

    #[test]
    fn random_between_zero_properties(n in 2..1000, max in 0..1_000_000) {
      let mut rng = rand::thread_rng();
      let max = std::cmp::max(max, n);
      let vecs = random_between_zero(n as usize, max as usize, &mut rng);
      prop_assert_eq!(vecs.iter().sum::<BigInt>(), BigInt::from(0));
      prop_assert!(vecs.iter().all(|v| !v.is_zero()));
      prop_assert_eq!(vecs.len(), n as usize);
    }

    #[test]
    fn sum_to_zero_vector(n in 2..1000, max in 0..1_000_000) {
      let mut rng = rand::thread_rng();
      let max = std::cmp::max(max, n);
      let vecs = random_vectors(n as usize, max as usize, &mut rng);
      prop_assert_eq!(vecs.into_iter().sum::<Vector<BigRational,2>>(), Vector::zero())
    }
  }
}
