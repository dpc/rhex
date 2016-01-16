use super::actor;

pub const ELF_STATS : actor::Stats = actor::Stats {
    int : 3,
    dex : 4,
    str_ : 2,
    max_hp : 15,
    max_mp : 20,
    max_sp : 10,
    ac: 0,
    ev: 2,
    infravision : 3,
    vision : 15,
};

pub const HUMAN_STATS : actor::Stats =  actor::Stats {
    int : 3,
    dex : 3,
    str_ : 3,
    max_hp : 15,
    max_mp : 15,
    max_sp : 15,
    ac: 1,
    ev: 1,
    infravision : 1,
    vision : 10,
};


pub const DWARF_STATS : actor::Stats =  actor::Stats {
    int : 3,
    dex : 2,
    str_ : 4,
    max_hp : 15,
    max_mp : 10,
    max_sp : 20,
    ac: 2,
    ev: 0,
    infravision : 2,
    vision : 10,
};


pub const RAT_STATS : actor::Stats = actor::Stats {
    int : 1,
    dex : 2,
    str_ : 2,
    max_hp : 5,
    max_mp : 5,
    max_sp : 5,
    ac: 0,
    ev: 2,
    infravision : 1,
    vision : 6,
};

pub const GOBLIN_STATS : actor::Stats = actor::Stats {
    int : 2,
    dex : 2,
    str_ : 3,
    max_hp : 15,
    max_mp : 10,
    max_sp : 10,
    ac: 0,
    ev: 1,
    infravision : 2,
    vision : 10,
};
