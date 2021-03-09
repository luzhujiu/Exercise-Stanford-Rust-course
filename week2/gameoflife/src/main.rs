extern crate nannou;
use nannou::prelude::*;
use nannou::ui::prelude::*;
use nannou::prelude::Rect;

use grid::Grid;
use ui::{update_ui};

pub mod grid;
pub mod ui;

pub struct Model {
    grid: Grid,
    duration: f32,
    ui: Ui,
    ids: Ids,
    is_active: bool,
    counter: usize,
    num_of_rows: usize,
}

widget_ids! {
    struct Ids {
        start,
        duration,
        num_of_rows,
        reset,
    }
}


impl Grid {
    fn step(&self) -> Grid {
        let mut new_grid = Grid::new(self.num_rows, self.num_cols);
        for row in 0..self.num_rows {
            for col in 0..self.num_cols {
                let current = self.get(row, col).unwrap();
                let count = self.count_alive(row as isize, col as isize);
                if current == 1 {
                    if count == 2 || count == 3 {
                        new_grid.set(row, col, 1);
                    }
                } else {
                    if count == 3 {
                        new_grid.set(row, col, 1);
                    }
                }
            }
        }
        return new_grid;
    }

    fn count_alive(&self, row: isize, col: isize) -> usize {
        let _surrounding = vec![(row-1,col-1),(row-1, col),(row-1,col+1),
                               (row, col-1),(row, col+1),
                               (row+1,col-1),(row+1,col),(row+1,col+1)];
        
        let surrounding = _surrounding.iter()
        .filter(|&cood| cood.0 >= 0 && cood.1 >= 0 
            && cood.0 < self.num_rows as isize && cood.1 < self.num_cols as isize)
        .map(|&cood| (cood.0 as usize, cood.1 as usize)).collect::<Vec<(usize, usize)>>();
        
        let mut count = 0;
        for (x, y) in surrounding {
            let value = self.get(x, y).unwrap();
            if value == 1 {
                count += 1;
            }
        }
        return count;
    }
}

fn main() {
    nannou::app(model).update(update).run();
}

fn model(_app: &App) -> Model { 
    _app.set_loop_mode(LoopMode::rate_fps(60.0));

    let _window = _app
        .new_window()
        .title("game of life")
        .mouse_pressed(mouse_pressed)
        .view(view)
        .build()
        .unwrap();

    let mut ui = _app.new_ui().build().unwrap();
    let ids = Ids::new(ui.widget_id_generator());
    let num_of_rows = 10;

    Model {
       grid: Grid::new(num_of_rows, num_of_rows),
       duration: 1.0, 
       num_of_rows,
       ui,
       ids,
       is_active: false,
       counter: 0,
    }
}

fn mouse_pressed(_app: &App, _model: &mut Model, _mouse: MouseButton) {
    if _model.is_active {
        return;
    }
    let (r, step) = get_grid_rect(_app, _model);
    let position = _app.mouse.position();

    if !r.contains(position) {
        return;
    } 
    
    let (left, top) = (r.left(), r.top());
    let (x, y) =  (position.x - left, top - position.y);
    let col = (x / step as f32).trunc() as usize;
    let row = (y / step as f32).trunc() as usize;

    if _mouse == MouseButton::Left {
        _model.grid.set(row, col, 1);
    } else if _mouse == MouseButton::Right {
        _model.grid.set(row, col, 0);
    }

}

const fps: f32 = 60.0;

fn update(_app: &App, _model: &mut Model, _update: Update) {
    if _model.counter as f32 >= fps * _model.duration {
        _model.counter = 0;
        if _model.is_active {
            let new_grid = _model.grid.step();
            _model.grid = new_grid;
        }
    } else {
        _model.counter += 1;
    }

    update_ui(_app, _model, _update);
}

fn get_grid_rect(_app: &App, _model: &Model) -> (Rect, usize) {
    let win = _app.window_rect().pad(30.0);
    let num_of_rows = _model.num_of_rows;
    let width = (win.w() * 0.8).round() as usize;
    let height = win.h().round() as usize;
    let len = std::cmp::min(width, height);
    let step = len / num_of_rows;
    let r = Rect::from_w_h(len as f32, len as f32).top_left_of(win);
    return (r, step);
}

fn view(_app: &App, _model: &Model, frame: Frame){
    frame.clear(PLUM);
    let draw = _app.draw();
    let (r, step) = get_grid_rect(_app, _model);

    draw.rect()
        .xy(r.xy())
        .wh(r.wh())
        .color(rgba(0.3, 0.4, 0.7, 0.5));
    
    let (top, bottom, left, right) = (r.top(), r.bottom(), r.left(), r.right());
    let num_of_rows = _model.num_of_rows;

    for i in 0..=num_of_rows {
        let from = pt2(left + i as f32 * step as f32, top);
        let to = pt2(left + i as f32 * step as f32, bottom);
        draw.line()
            .start(from)
            .end(to)
            .weight(4.0)
            .color(STEELBLUE);
    
        let from = pt2(left, top - i as f32 * step as f32);
        let to = pt2(right, top - i as f32 * step as f32);
        draw.line()
            .start(from)
            .end(to)
            .weight(4.0)
            .color(STEELBLUE);    

    }

    for row in 0..num_of_rows {
            let _top = top - row as f32 * step as f32;
        for col in 0..num_of_rows {
            let _left = left + col as f32 * step as f32;
            let (x, y) = (_left + step as f32 / 2.0, _top - step as f32 / 2.0); 
            let value = _model.grid.get(row, col).unwrap();
            if value == 1 {
                draw.ellipse()
                .x_y(x,y)
                .radius(step as f32 * 0.9 / 2.0)
                .color(RED);
            } 
        }
    }
    
    let win = _app.window_rect().pad(25.0);
    let r = Rect::from_w_h(200.0,60.0).top_right_of(win);
    
    draw.text(&format!("duration = {}\n num_of_rows = {}", _model.duration, _model.num_of_rows))
        .x_y(r.x_y().0.into(), 0.0)
        .align_text_bottom()
        .color(STEELBLUE)
        .font_size(20);

    draw.to_frame(_app, &frame).unwrap();
    _model.ui.draw_to_frame(_app, &frame).unwrap();
}
