use gdk;
use gdk::glib::Type;
use gtk::prelude::*;
use gtk::{
    Adjustment, Application, ApplicationWindow, Box, Button, CellRendererText, Entry, Grid, Label,
    ListStore, Orientation, ScrolledWindow, TreeView, TreeViewColumn, Window, WindowType,
};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::from_utf8;

#[derive(Debug)]
struct ResponseInterval {
    id: i32,
    interval: Vec<i32>,
    sent_time: String,
    recieved_time: String,
    server_recieved_time: String,
    server_response_time: String,
    pi_result: f64,
}

fn pi_calc(interval: &Vec<i32>) -> f64 {
    let mut pi = 0.0;
    for i in interval[0]..=interval[1] {
        let upper = -1.0_f64.powi(i);
        let lower = 2.0 * i as f64 + 1.0;

        pi += upper / lower;
    }
    return pi * 4.0;
}

fn send_request(address: &str, message: &str, num_calls: i32) -> Vec<ResponseInterval> {
    let mut results = Vec::new();

    for i in 0..=num_calls {
        match TcpStream::connect(address) {
            Ok(mut stream) => {
                println!("Successfully connected to server");

                let m = format!("{},{}", i, message);
                let msg = m.as_bytes();

                // send message
                stream.write(msg).unwrap();
                let sent_time = chrono::Local::now().to_string();
                println!("Sent message, awaiting reply...");

                let mut data = [0 as u8; 256];
                match stream.read(&mut data) {
                    Ok(size) => {
                        let received_time = chrono::Local::now().to_string();
                        if let Ok(text) = from_utf8(&data[0..size]) {
                            let request_to_vec = text.split('|').collect::<Vec<&str>>();

                            if request_to_vec.len() != 4 {
                                println!("Invalid response from server");
                                continue;
                            }

                            let interval_vec: Vec<i32> = request_to_vec[1]
                                .trim_start_matches('[')
                                .trim_end_matches(']')
                                .split(',')
                                .map(|s| s.trim().parse().unwrap())
                                .collect();
                            let pi_result = pi_calc(&interval_vec);
                            results.push(ResponseInterval {
                                id: request_to_vec[0].parse::<i32>().unwrap(),
                                interval: interval_vec,
                                sent_time: sent_time,
                                recieved_time: received_time,
                                server_recieved_time: request_to_vec[2].to_string(),
                                server_response_time: request_to_vec[3].to_string(),
                                pi_result: pi_result,
                            });
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to receive data: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to connect: {}", e);
            }
        }
    }

    println!("Results: {:?}", results);

    results
}

fn create_and_fill_model(results: &[ResponseInterval]) -> ListStore {
    let store = ListStore::new(&[
        Type::I32,
        Type::STRING,
        Type::STRING,
        Type::STRING,
        Type::STRING,
        Type::STRING,
        Type::F64,
    ]);

    for result in results {
        let iter = store.append();
        store.set_value(&iter, 0, &result.id.to_value());
        store.set_value(&iter, 1, &format!("{:?}", result.interval).to_value());
        store.set_value(&iter, 2, &result.sent_time.to_value());
        store.set_value(&iter, 3, &result.recieved_time.to_value());
        store.set_value(&iter, 4, &result.server_recieved_time.to_value());
        store.set_value(&iter, 5, &result.server_response_time.to_value());
        store.set_value(&iter, 6, &result.pi_result.to_value());
    }

    store
}

fn append_text_column(tree: &TreeView, title: &str, id: i32) {
    let column = TreeViewColumn::new();
    let cell = CellRendererText::new();

    CellLayoutExt::pack_start(&column, &cell, true);
    CellLayoutExt::add_attribute(&column, &cell, "text", id);
    column.set_title(title);
    tree.append_column(&column);
}

fn main() {
    let app = Application::builder()
        .application_id("cliente.servidor.multithread")
        .build();

    app.connect_activate(|app| {
        let win = ApplicationWindow::builder()
            .application(app)
            .default_width(320)
            .default_height(150)
            .title("Cliente-Servidor Multithread")
            .build();

        let vbox = Box::new(Orientation::Vertical, 0);
        let hbox = Box::new(Orientation::Horizontal, 0);
        vbox.pack_start(&hbox, true, true, 0);

        let grid = Grid::new();
        grid.set_row_spacing(5);
        grid.set_column_spacing(5);
        hbox.pack_start(&grid, true, false, 0);

        let label = Label::new(Some("Digite o número de requisições:"));
        let input = Entry::new();
        grid.attach(&label, 0, 0, 2, 1);
        grid.attach(&input, 0, 1, 1, 1);

        let submit_button = Button::with_label("Submit");
        submit_button.connect_clicked(move |_| {
            let buffer = input.buffer();
            let text = buffer.text();
            if let Ok(number) = text.parse::<i32>() {
                println!("Number entered: {}", number);
                let results = send_request("127.0.0.1:7878", "interval", number);
                let model = create_and_fill_model(&results);
                let tree = TreeView::with_model(&model);

                append_text_column(&tree, "ID", 0);
                append_text_column(&tree, "Interval", 1);
                append_text_column(&tree, "Sent Time", 2);
                append_text_column(&tree, "Received Time", 3);
                append_text_column(&tree, "Server Received Time", 4);
                append_text_column(&tree, "Server Response Time", 5);
                append_text_column(&tree, "Pi Result", 6);

                let window = Window::new(WindowType::Toplevel);
                window.set_title("Results");
                window.set_default_size(800, 600);

                let scroll = ScrolledWindow::new(None::<&Adjustment>, None::<&Adjustment>);
                scroll.add(&tree);

                window.add(&scroll);
                window.show_all();

                // Call your function here
            } else {
                println!("Not a valid number");
            }
        });
        grid.set_margin_top(24);
        grid.attach(&submit_button, 1, 1, 1, 1);

        win.add(&vbox);

        win.show_all();
    });

    app.run();
}
