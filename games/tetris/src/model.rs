use rust_pixel::event::{Event, KeyCode};
use log::debug;
//use rand::prelude::*;
use tetris_lib::{
    ai::*,
    cell::{Move, MoveRet, TetrisCell},
    constant::*,
};
use std::any::Any;
use rust_pixel::{
    context::Context,
    event::{event_emit, timer_fire},
    game::Model,
    //timer_cancel, timer_fire, timer_register,
    util::Rand,
};

//https://harddrop.com/wiki/T-Spin_Triple_Setups
//https://katyscode.wordpress.com/2012/10/13/tetris-aside-coding-for-t-spins/

/*pub enum TetrisState {
    Normal,
    OverSelf,
    OverBorder,
}*/

pub struct TetrisModel {
    pub cells: [TetrisCell; 2],
    pub block_queue: [i8; BLKQUEUE as usize],
    pub trand: Rand,
    pub tai: TetrisAi,
    pub timeout_auto: f32,
    pub timeout_ai: f32,
}

impl TetrisModel {
    pub fn new() -> Self {
        let c: [TetrisCell; 2] = [TetrisCell::new(0), TetrisCell::new(1)];
        Self {
            cells: c,
            block_queue: [0i8; BLKQUEUE as usize],
            trand: Rand::new(),
            tai: TetrisAi::new(),
            timeout_auto: 0.0,
            timeout_ai: 0.0,
        }
    }

    fn random_block_queue(&mut self, seed: u32) {
        self.trand.srand(seed as u64);
        self.trand.srand_now();
        for i in 0..BLKQUEUE {
            self.block_queue[i as usize] = (self.trand.rand() % 7) as i8;
        }
    }

    fn reset(&mut self) {
        self.random_block_queue(0);
        self.cells[0].reset(&self.block_queue);
        self.cells[1].reset(&self.block_queue);
        event_emit("Tetris.RedrawGrid");
    }

    pub fn act(&mut self, index: usize, d: Move, _context: &mut Context) {
        if d == Move::Restart {
            self.reset();
        }
        if self.cells[0].core.game_over || self.cells[1].core.game_over {
            return;
        }
        match d {
            Move::TurnCw | Move::TurnCcw => {
                if self.cells[index].move_block(d, false) == MoveRet::Normal {
                    self.cells[index].make_shadow();
                } else {
                    //开始尝试左右移动再转...
                    let cmds = ["L", "LL", "R", "RR"];
                    for c in cmds {
                        if self.cells[index].help_turn(d, c) {
                            return;
                        }
                    }
                }
            }
            Move::DropDown => {
                timer_fire(&format!("fall{}", index), 0);
                debug!("fire fall{}", index);
            }
            Move::Down => {
                if self.cells[index].move_block(d, false) == MoveRet::ReachBottom {
                    self.cells[index].next_block(&self.block_queue, false, false);
                }
            }
            Move::Left | Move::Right => {
                self.cells[index].move_block(d, false);
                self.cells[index].make_shadow();
            }
            Move::Save => {
                self.cells[index].save_block(&self.block_queue, false);
                self.cells[index].make_shadow();
            }
            _ => {}
        }
    }
}

impl Model for TetrisModel {
    fn init(&mut self, _context: &mut Context) {
        self.reset();
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Key(key) => {
                    let mut d: Option<Move> = None;
                    match key.code {
                        KeyCode::Char(' ') => {
                            d = Some(Move::DropDown);
                        }
                        KeyCode::Char('o') => d = Some(Move::TurnCcw),
                        KeyCode::Char('i') => d = Some(Move::TurnCw),
                        KeyCode::Char('j') => d = Some(Move::Left),
                        KeyCode::Char('k') => d = Some(Move::Down),
                        KeyCode::Char('l') => d = Some(Move::Right),
                        KeyCode::Char('s') => d = Some(Move::Save),
                        KeyCode::Char('r') => d = Some(Move::Restart),
                        _ => {}
                    }
                    if d != None {
                        self.act(0, d.unwrap(), context);
                    }
                }
                _ => {}
            }
        }
        context.input_events.clear();
    }

    fn handle_event(&mut self, _context: &mut Context, _dt: f32) {}

    fn handle_timer(&mut self, _context: &mut Context, _dt: f32) {
        for i in 0..2 as usize {
            if self.cells[i].core.game_over {
                continue;
            }
            self.cells[i].timer_process(&self.block_queue);
            if self.cells[i].core.attack[0] != 0 {
                self.cells[1 - i].attacked(
                    &mut self.trand,
                    self.cells[i].core.attack[0],
                    self.cells[i].core.attack[1],
                );
                self.cells[1 - i].make_shadow();
                self.cells[i].core.attack[0] = 0;
            }
        }
    }

    fn handle_auto(&mut self, context: &mut Context, dt: f32) {
        if self.tai.work2idx >= 0 {
            self.tai.get_ai_act(&self.block_queue, &mut self.cells[1]);
        }

        if self.timeout_auto > 0.4 {
            self.timeout_auto = 0.0;
            self.act(0, Move::Down, context);
            self.cells[0].core.dump_debug();
        } else {
            self.timeout_auto += dt;
        }

        if self.timeout_ai > 0.1 {
            self.timeout_ai = 0.0;
            let c = self.tai.get_ai_act(&self.block_queue, &mut self.cells[1]);
            debug!("getAiAct::{}", c);
            let d: Option<Move> = match c {
                'S' => Some(Move::Save),
                'T' => Some(Move::TurnCw),
                'W' => Some(Move::DropDown),
                'L' => Some(Move::Left),
                'R' => Some(Move::Right),
                _ => None,
            };
            if d != None {
                self.act(1, d.unwrap(), context);
            }
        } else {
            self.timeout_ai += dt;
        }
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}
