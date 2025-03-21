use crate::gdk::Texture;
use chrono::prelude::*;
use glib::timeout_add_local;
use gstreamer::Bin;
use gstreamer::Element;
use gstreamer::Pipeline;
use gstreamer::State;
use gstreamer::prelude::*;
use gtk4::gdk::Display;
use gtk4::glib;
use gtk4::{Application, ApplicationWindow, Box as GtkBox, Button, CssProvider, Picture, Spinner};
use gtk4::{gdk, prelude::*};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use gstreamer::glib::property::PropertySet;

mod gst_utils;
mod picture;

use crate::gst_utils::{link_tee_branch, unlink_tee_branch};

struct Config {
    camera: CameraConfig,
}

#[derive(Clone)]
struct CameraConfig {
    width: i32,
    height: i32,
    fps: i32,
    path: String,
}

struct AppState {
    pipeline: Pipeline,
    tee: Element,
    recording_branch: Option<Bin>,
    is_recording: bool,
}

impl AppState {
    fn new(pipeline: Pipeline) -> Self {
        let tee = pipeline
            .by_name("t")
            .expect("Не удалось найти элемент tee в pipeline");

        Self {
            pipeline,
            tee,
            recording_branch: None,
            is_recording: false,
        }
    }

    fn start_recording(&mut self, file_path: &str) {
        if self.is_recording {
            return;
        }

        println!("Начинаем запись в файл: {}", file_path);

        // Обновленная строка для branch записи с улучшенными параметрами кодирования
        let branch_str = format!(
            "queue ! videoconvert ! x264enc tune=zerolatency speed-preset=superfast \
            key-int-max=30 ! video/x-h264,profile=main ! mp4mux streamable=true \
            fragment-duration=1 ! filesink location={} sync=false",
            file_path
        );

        println!("Создаем branch с настройками: {}", branch_str);

        match gstreamer::parse::bin_from_description(&branch_str, true) {
            Ok(branch) => match link_tee_branch(&self.pipeline, &self.tee, &branch) {
                Ok(_) => {
                    println!("Branch успешно подключен");
                    self.recording_branch = Some(branch);
                    self.is_recording = true;
                }
                Err(e) => println!("Ошибка при подключении branch: {:?}", e),
            },
            Err(e) => println!("Ошибка создания branch: {:?}", e),
        }
    }

    fn stop_recording(&mut self) {
        if !self.is_recording {
            return;
        }

        if let Some(branch) = self.recording_branch.take() {
            println!("Останавливаем запись...");

            // Сначала отправляем EOS для branch записи
            if let Some(sink_pad) = branch.static_pad("sink") {
                sink_pad.send_event(gstreamer::event::Eos::new());
            }

            // Увеличиваем время ожидания для корректного завершения записи
            std::thread::sleep(std::time::Duration::from_millis(500));

            // Переводим branch в состояние NULL
            let _ = branch.set_state(State::Null);

            // Отключаем branch записи
            unlink_tee_branch(
                &self.pipeline,
                &self.tee,
                &branch,
                Box::new(|| {
                    println!("Branch записи отключен");
                }),
            );

            // Перезапускаем pipeline
            let _ = self.pipeline.set_state(State::Null);
            std::thread::sleep(std::time::Duration::from_millis(100));
            let _ = self.pipeline.set_state(State::Playing);

            // Ждем, пока pipeline действительно перейдет в состояние PLAYING
            let _ = self.pipeline.state(gstreamer::ClockTime::from_seconds(1));

            println!("Pipeline перезапущен");
        }

        self.is_recording = false;
    }
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_file(&gtk4::gio::File::for_path("src/style.css"));

    gtk4::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn handle_pipeline_messages(bus: &gstreamer::Bus, pipeline: &gstreamer::Pipeline) -> bool {
    while let Some(msg) = bus.pop() {
        match msg.view() {
            gstreamer::MessageView::Error(err) => {
                println!(
                    "Error from {:?}: {} ({:?})",
                    err.src().map(|s| s.name()),
                    err.error(),
                    err.debug()
                );

                // Проверяем, от какого элемента пришла ошибка
                if let Some(src) = err.src() {
                    // Игнорируем ошибки от элементов записи
                    if src.name().as_str().starts_with("filesink")
                        || src.name().as_str().starts_with("qtmux")
                        || src.name().as_str().starts_with("x264enc")
                        || src.name().as_str().starts_with("queue")
                    {
                        println!("Игнорируем ошибку от элемента: {}", src.name());
                        return true;
                    }
                }

                // Для других ошибок пытаемся восстановить pipeline
                let _ = pipeline.set_state(State::Null);
                std::thread::sleep(std::time::Duration::from_millis(100));
                let _ = pipeline.set_state(State::Playing);
                return true;
            }
            gstreamer::MessageView::StateChanged(state_changed) => {
                // Логируем изменения состояния для отладки
                if let Some(element) = state_changed.src() {
                    println!(
                        "State changed for {}: {:?} -> {:?}",
                        element.name(),
                        state_changed.old(),
                        state_changed.current()
                    );
                }
            }
            gstreamer::MessageView::Eos(_) => {
                // Игнорируем EOS от branch записи
                println!("Получен EOS, игнорируем");
                return true;
            }
            _ => (),
        }
    }
    true
}

fn main() {
    let app = Application::new(Some("com.example.MyGTKApp"), Default::default());
    app.connect_startup(|_| load_css());

    let config = Config {
        camera: CameraConfig {
            width: 720,
            height: 480,
            fps: 25,
            path: String::from("src/media/"),
        },
    };

    let camera_config = config.camera.clone();

    gstreamer::init().expect("Не удалось инициализировать GStreamer");
    gstgtk4::plugin_register_static().expect("Failed to register gstgtk4 plugin");

    let media_path = Path::new(&config.camera.path);
    if !media_path.exists() {
        fs::create_dir_all(media_path).expect("Failed to create media directory");
    }

    let pipeline_str = format!(
        "v4l2src device=/dev/video0 ! image/jpeg,width={},height={},framerate={}/1 ! 
        jpegdec ! videoconvert ! tee name=t allow-not-linked=true ! 
        queue max-size-buffers=2 leaky=downstream ! videoconvert ! 
        gtk4paintablesink name=sink1 sync=false",
        config.camera.width, config.camera.height, config.camera.fps
    );

    let pipeline = gstreamer::parse::launch(&pipeline_str)
        .expect("Can not create GStreamer pipeline")
        .dynamic_cast::<Pipeline>()
        .expect("Can not cast to Pipeline");

    let pipeline_weak = pipeline.downgrade();

    app.connect_activate(move |app| {
        let window = ApplicationWindow::new(app);
        window.set_title(Some("My GTK App"));
        window.set_default_size(1024, 600);
        window.set_decorated(false);

        let hbox = GtkBox::new(gtk4::Orientation::Horizontal, 5);
        hbox.set_halign(gtk4::Align::Center);
        hbox.set_valign(gtk4::Align::Center);
        hbox.add_css_class("screen_box");

        let vbox1 = GtkBox::new(gtk4::Orientation::Vertical, 5);
        vbox1.set_hexpand(true);
        vbox1.set_vexpand(true);

        let display_window = GtkBox::new(gtk4::Orientation::Vertical, 5);
        display_window.set_hexpand(true);
        display_window.set_vexpand(true);
        display_window.set_size_request(720, 480);

        let vbox3 = GtkBox::new(gtk4::Orientation::Vertical, 5);
        vbox3.set_hexpand(true);
        vbox3.set_vexpand(true);

        let button1 = Button::with_label("Сканер Частоты");
        let button2 = Button::with_label("Ввод Позывного");
        let button3 = Button::with_label("Бинд Фраза");
        let button_rec = Button::with_label("Запись видео");

        // Устанавливаем фиксированный размер для всех кнопок
        button1.set_size_request(120, 80);
        button2.set_size_request(120, 80);
        button3.set_size_request(120, 80);
        button_rec.set_size_request(120, 80);

        // Добавляем дополнительный CSS класс для фиксированной ширины
        button1.add_css_class("fixed-width");
        button2.add_css_class("fixed-width");
        button3.add_css_class("fixed-width");
        button_rec.add_css_class("fixed-width");

        button_rec.add_css_class("rec_btn");
        button1.add_css_class("custom-button");
        button2.add_css_class("custom-button");
        button3.add_css_class("custom-button");

        button1.set_hexpand(true);
        button1.set_vexpand(true);
        button2.set_hexpand(true);
        button2.set_vexpand(true);
        button3.set_hexpand(true);
        button3.set_vexpand(true);
        button_rec.set_hexpand(true);
        button_rec.set_vexpand(true);

        let gtksink = pipeline
            .by_name("sink1")
            .expect("Can not get gtk4paintablesink element");

        let paintable = gtksink.property::<gdk::Paintable>("paintable");
        let picture = Picture::new();
        picture.set_paintable(Some(&paintable));

        vbox1.append(&button1);
        vbox1.append(&button2);
        display_window.append(&picture);
        vbox3.append(&button3);
        vbox3.append(&button_rec);

        hbox.append(&vbox1);
        hbox.append(&display_window);
        hbox.append(&vbox3);

        window.set_child(Some(&hbox));
        window.show();

        pipeline
            .set_state(State::Playing)
            .expect("Unable to set the pipeline to the `Playing` state");

        let picture = Rc::new(RefCell::new(picture));

        let spinner_active = Arc::new(AtomicBool::new(false));

        // Добавляем структуру для хранения ID таймера
        let timeout_id = Rc::new(RefCell::new(None::<glib::SourceId>));

        button1.connect_clicked({
            let display_window = display_window.clone();
            let picture = picture.clone();
            let _button1 = button1.clone();
            let button2 = button2.clone();
            let button3 = button3.clone();
            let button_rec = button_rec.clone();
            let spinner_active = spinner_active.clone();
            let timeout_id = timeout_id.clone();

            move |button| {
                if spinner_active.load(Ordering::SeqCst) {
                    // Если нажата кнопка отмены
                    if button.label().map_or(false, |l| l == "Отмена") {
                        // Отменяем таймер если он существует
                        if let Some(id) = timeout_id.take() {
                            id.remove();
                            timeout_id.set(None);
                        }

                        // Восстанавливаем интерфейс
                        button.set_label("Сканер Частоты");
                        button2.set_label("Ввод Позывного");
                        button3.set_label("Бинд Фраза");
                        button_rec.set_label("Запись видео");

                        button2.set_sensitive(true);
                        button3.set_sensitive(true);
                        button_rec.set_sensitive(true);

                        // Возвращаем картинку с камеры
                        if let Some(spinner) = display_window.first_child() {
                            display_window.remove(&spinner);
                            display_window.append(&*picture.borrow());
                        }

                        spinner_active.store(false, Ordering::SeqCst);
                    }
                    return;
                }

                println!("Button 1 clicked");

                spinner_active.store(true, Ordering::SeqCst);

                let current_picture = picture.borrow().clone();

                button.set_label("Отмена");
                button2.set_label("");
                button3.set_label("");
                button_rec.set_label("");

                button2.set_sensitive(false);
                button3.set_sensitive(false);
                button_rec.set_sensitive(false);

                let spinner_box = GtkBox::new(gtk4::Orientation::Vertical, 0);
                spinner_box.set_hexpand(true);
                spinner_box.set_vexpand(true);
                spinner_box.set_halign(gtk4::Align::Center);
                spinner_box.set_valign(gtk4::Align::Center);

                let spinner = Spinner::new();
                spinner.set_size_request(100, 100);
                spinner.set_halign(gtk4::Align::Center);
                spinner.set_valign(gtk4::Align::Center);
                spinner.start();

                spinner_box.append(&spinner);

                display_window.remove(&*picture.borrow());
                display_window.append(&spinner_box);

                let display_window = display_window.clone();
                let picture = picture.clone();
                let button = button.clone();
                let button2 = button2.clone();
                let button3 = button3.clone();
                let button_rec = button_rec.clone();
                let spinner_active = spinner_active.clone();

                // Сохраняем ID таймера
                let source_id = glib::timeout_add_local(Duration::from_secs(2), move || {
                    display_window.remove(&spinner_box);

                    *picture.borrow_mut() = current_picture.clone();
                    display_window.append(&*picture.borrow());

                    button.set_label("Сканер Частоты");
                    button2.set_label("Ввод Позывного");
                    button3.set_label("Бинд Фраза");
                    button_rec.set_label("Запись видео");

                    button2.set_sensitive(true);
                    button3.set_sensitive(true);
                    button_rec.set_sensitive(true);

                    spinner_active.store(false, Ordering::SeqCst);

                    glib::ControlFlow::Break
                });

                timeout_id.set(Some(source_id));
            }
        });

        button2.connect_clicked({
            let display_window = display_window.clone();
            let picture = picture.clone();
            move |_| {
                println!("Button 2 clicked");
                let file = gtk4::gio::File::for_path("src/images/cat.jpg");
                let texture = Texture::from_file(&file).expect("Failed to load image");
                let new_picture = Picture::new();
                new_picture.set_paintable(Some(&texture));
                let _ = display_window.remove(&*picture.borrow());
                *picture.borrow_mut() = new_picture;
                display_window.append(&*picture.borrow());
            }
        });

        button3.connect_clicked({
            let display_window = display_window.clone();
            let picture = picture.clone();
            move |_| {
                println!("Button 3 clicked");
                let file = gtk4::gio::File::for_path("src/images/tiger.jpg");
                let texture = Texture::from_file(&file).expect("Failed to load image");
                let new_picture = Picture::new();
                new_picture.set_paintable(Some(&texture));
                let _ = display_window.remove(&*picture.borrow());
                *picture.borrow_mut() = new_picture;
                display_window.append(&*picture.borrow());
            }
        });

        let app_state = Rc::new(RefCell::new(AppState::new(pipeline.clone())));

        button_rec.connect_clicked({
            let app_state = app_state.clone();
            let camera_path = camera_config.path.clone();
            move |button| {
                let mut state = app_state.borrow_mut();
                if !state.is_recording {
                    let now = Utc::now();
                    let file_path =
                        format!("{}{}.mp4", camera_path, now.format("%Y-%m-%d|%H:%M:%S"));
                    state.start_recording(&file_path);
                    button.add_css_class("recording");
                    button.set_label("Стоп запись");
                } else {
                    state.stop_recording();
                    button.remove_css_class("recording");
                    button.set_label("Запись видео");
                }
            }
        });

        let bus = pipeline.bus().expect("Не удалось получить шину pipeline");
        let app_weak = app.downgrade();

        let pipeline_weak = pipeline_weak.clone();
        timeout_add_local(Duration::from_millis(100), move || {
            let pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => return glib::ControlFlow::Break,
            };

            if !handle_pipeline_messages(&bus, &pipeline) {
                if let Some(app) = app_weak.upgrade() {
                    app.quit();
                }
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
    });

    app.run();
}
