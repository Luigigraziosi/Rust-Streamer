use rdev::{listen, EventType, Button};
use std::thread;
use std::ptr::null_mut;
use winapi::um::wingdi::{MoveToEx, LineTo, CreatePen, SelectObject, DeleteObject, RGB};
use winapi::um::winuser::{GetDC, ReleaseDC, HWND_DESKTOP};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;

enum Shape {
    Cross((i32, i32)),
    Square((i32, i32), (i32, i32)),
}



// Definiamo una struttura per contenere lo stato
// struct State {
//     p1: (i32, i32),
//     p2: (i32, i32),
//     click: i32,
// }

struct State {
    p1: (i32, i32),
    p2: (i32, i32),
    click: i32,
    should_exit: bool, // Flag per indicare se dobbiamo uscire
}

fn screen(state: Arc<Mutex<State>>, tx: mpsc::Sender<()>) {
    let mut current_position: (i32, i32) = (0, 0);

    if let Err(error) = listen(move |event| {
        let mut state = state.lock().unwrap(); // Blocchiamo il mutex per accedere ai dati

        draw_shapes(state.click, state.p1, state.p2); // Usando i valori aggiornati dallo stato
        match event.event_type {
            EventType::MouseMove { x, y } => {
                current_position = (x as i32, y as i32);
            }
            EventType::ButtonPress(button) => {
                if button == Button::Left {
                    state.click += 1;
                    if state.click >= 3 {
                        state.should_exit = true; // Impostiamo il flag di uscita
                        if let Err(send_error) = tx.send(()) {
                            eprintln!("Failed to send exit signal: {:?}", send_error);
                            return;
                        }
                        return; // Uscita anticipata dal thread
                    }
                    current_position = clamp_point(current_position, 1920, 1080);
                    if state.click == 1 {
                        state.p1 = current_position;
                        draw_shapes(state.click, state.p1, state.p2);
                    } else if state.click == 2 {
                        state.p2 = current_position;
                        draw_shapes(1, state.p1, state.p2);
                        draw_shapes(state.click, state.p1, state.p2);
                    }
                    println!(
                        "Mouse button {:?} pressed, last position is {}, {}",
                        button, current_position.0, current_position.1
                    );
                }
            }
            _ => {}
        }
    }) {
        eprintln!("Error occurred while listening: {:?}", error);
    }
}


pub fn wrapper_schermo()-> ((i32, i32), (i32, i32)){
    let state = Arc::new(Mutex::new(State {
        p1: (0, 0),
        p2: (0, 0),
        click: 0,
        should_exit: false,
    }));

    let (tx, rx) = mpsc::channel();
    
    let state_clone = Arc::clone(&state);
    thread::spawn(move || {
        screen(state_clone, tx);
    });

    // Aspettiamo che il thread di ascolto invii un segnale di uscita
    rx.recv().unwrap();
    let state = state.lock().unwrap();
    (state.p1, state.p2)
}

fn clamp_point(point: (i32, i32), max_width: i32, max_height: i32) -> (i32, i32) {
    let clamped_x = point.0.clamp(0, max_width);
    let clamped_y = point.1.clamp(0, max_height);
    (clamped_x, clamped_y)
}

fn draw_shapes(click: i32, p1: (i32, i32), p2: (i32, i32)) {
    match click {
        1 => draw_shape(Shape::Cross(p1)),
        2 => {
            draw_shape(Shape::Cross(p1));
            draw_shape(Shape::Square(p1, p2));
            draw_shape(Shape::Cross(p2));
        }
        _ => {}
    }
}

fn draw_shape(shape: Shape) {
    unsafe {
        let hdc = GetDC(HWND_DESKTOP);
        if hdc.is_null() {
            eprintln!("Failed to get device context");
            return;
        }

        let pen = CreatePen(0, 2, RGB(255, 0, 0)); // Penna rossa
        let old_pen = SelectObject(hdc, pen as _);

        match shape {
            Shape::Cross((x, y)) => {
                MoveToEx(hdc, x - 10, y, null_mut());
                LineTo(hdc, x + 10, y);
                MoveToEx(hdc, x, y - 10, null_mut());
                LineTo(hdc, x, y + 10);
            }
            Shape::Square((p1_x, p1_y), (p2_x, p2_y)) => {
                let x1 = p2_x - p1_x;
                let y1 = p2_y - p1_y;

                MoveToEx(hdc, p1_x, p1_y, null_mut());
                LineTo(hdc, p1_x + x1, p1_y);
                LineTo(hdc, p1_x + x1, p1_y + y1);
                LineTo(hdc, p1_x, p1_y + y1);
                LineTo(hdc, p1_x, p1_y);
            }
        }

        SelectObject(hdc, old_pen);
        DeleteObject(pen as _);
        ReleaseDC(HWND_DESKTOP, hdc);
    }
}
