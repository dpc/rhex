//! TODO: Move to `hex2d::algo::bfs` ?

/// Breadth First Search
pub mod bfs {

    use std::collections::ring_buf::RingBuf;
    use std::collections::HashMap;
    use std::collections::hash_map::Entry::{Occupied,Vacant};

    use hex2d as h2d;

    struct Visited {
        prev : h2d::Coordinate,
        dist : u32,
    }


    pub struct Traverser<F1, F2> where
        F1 : Fn(h2d::Coordinate) -> bool,
        F2 : Fn(h2d::Coordinate) -> bool
    {
        visited : HashMap<h2d::Coordinate, Visited>,
        to_traverse : RingBuf<h2d::Coordinate>,
        can_pass : F1,
        is_dest : F2,
        start :h2d::Coordinate,
    }

    impl<F1, F2> Traverser<F1, F2> where
        F1 : Fn(h2d::Coordinate) -> bool,
        F2 : Fn(h2d::Coordinate) -> bool
    {

        pub fn new(can_pass : F1, is_dest : F2, start: h2d::Coordinate) -> Traverser<F1, F2> {
            let mut to_traverse = RingBuf::new();
            to_traverse.push_back(start);

            let mut visited = HashMap::new();
            visited.insert(start, Visited{prev: start, dist: 0});

            Traverser {
                visited: visited,
                to_traverse: to_traverse,
                can_pass: can_pass,
                is_dest: is_dest,
                start: start,
            }
        }

        pub fn find(&mut self) -> Option<h2d::Coordinate> {

            loop {
                let pos = match self.to_traverse.pop_front() {
                    None => return None,
                    Some(coord) => coord,
                };

                // Traverse before returning, so `find` can be call subsequently
                // for more than just first answer
                if (self.can_pass)(pos) {

                    let &Visited{dist, ..} = self.visited.get(&pos).expect("BFS: Should have been visited already");

                    let dist = dist + 1;

                    for &npos in pos.neighbors().iter() {
                        match self.visited.entry(npos) {
                            Occupied(_) => { /* already visited */ }
                            Vacant(entry) => {
                                entry.insert(Visited{prev: pos, dist: dist});
                                self.to_traverse.push_back(npos);
                            }
                        }
                    }
                }

                if (self.is_dest)(pos) {
                    return Some(pos);
                }
            }
        }

        #[allow(dead_code)]
        pub fn backtrace(&self, pos : h2d::Coordinate) -> Option<h2d::Coordinate> {
            self.visited.get(&pos).map(|entry| entry.prev)
        }

        pub fn backtrace_last(&self, mut pos : h2d::Coordinate) -> Option<h2d::Coordinate> {
            loop {
                pos = match self.visited.get(&pos) {
                    None => return None,
                    Some(entry) => {
                        if entry.prev == self.start {
                            return Some(pos);
                        } else {
                            entry.prev
                        }
                    }
                }
            }
        }
    }
}

/// Very tricky, but (hopefully) fast and good enough, recursive LoS algorithm
pub mod los {

    use hex2d as h2d;
    use std::num::Int;

    fn los_rec<FOpaqueness, FVisible, I=i32>(
        opaqueness : &FOpaqueness,
        visible : &mut FVisible,
        light: I,
        p : h2d::Coordinate,
        main_dir : h2d::Direction,
        dir : Option<h2d::Direction>,
        pdir : Option<h2d::Direction>,
    ) where
        I : Int,
        FOpaqueness : Fn(h2d::Coordinate) -> I,
        FVisible : FnMut(h2d::Coordinate)
        {
            use hex2d::Angle::{Left, Right};

            visible(p);

            let mut light = light;
            let opaq = opaqueness(p);

            if opaq >= light {
                return;
            } else {
                light = light - opaq;
            }

            let neighbors = match (dir, pdir) {
                (Some(dir), Some(pdir)) => {

                    if main_dir == dir {
                        visible(p + (main_dir + Right));
                        visible(p + (main_dir + Left));
                    }

                    if dir == pdir {
                        vec!(dir)
                    } else {
                        vec!(dir, pdir)
                    }
                },
                (Some(dir), None) => {
                    if main_dir == dir {
                        visible(p + (main_dir + Right));
                        visible(p + (main_dir + Left));
                        vec!(dir, dir + Left, dir + Right)
                    } else {
                        visible((p + main_dir));
                        vec!(dir, main_dir)
                    }
                },
                _ => {
                    visible(p + main_dir);
                    visible(p + (main_dir + Left));
                    visible(p + (main_dir + Right));
                    vec!(main_dir, main_dir + Left, main_dir + Right)
                }
            };

            for &d in neighbors.iter() {
                let n = p + d;
                match dir {
                    Some(_) => los_rec::<FOpaqueness, FVisible, I>(opaqueness, visible, light, n, d, Some(d), dir),
                    None => los_rec::<FOpaqueness, FVisible, I>(opaqueness, visible, light, n, main_dir, Some(d), dir),
                }
            }
        }


    pub fn los<FOpaqueness, FVisible, I=i32>(
        opaqueness : &FOpaqueness,
        visible : &mut FVisible,
        light: I,
        pos : h2d::Coordinate,
        dirs : &[h2d::Direction],
    ) where
        I : Int,
        FOpaqueness : Fn(h2d::Coordinate) -> I,
        FVisible : FnMut(h2d::Coordinate)
        {
            for dir in dirs.iter() {
                los_rec::<FOpaqueness, FVisible, I>(opaqueness, visible, light, pos, *dir, None, None);
            }
        }
}
