use gpui::{AppContext as _, AsyncApp};

use super::Run;

impl Run {
    pub(super) async fn shot(&self, cx: &mut AsyncApp, name: &str) {
        let image = if name == "inbox" {
            self.sleep(cx, 150).await;
            let first = cx
                .update_window(self.window, |_, window, cx| {
                    window.refresh();
                    window.draw(cx).clear(cx);
                    window.render_to_image()
                })
                .expect("window alive");
            match first {
                Ok(first) => {
                    let row_text_pixels = [(46, 70), (83, 107), (120, 144)].map(|(top, bottom)| {
                        let mut count = 0;
                        for y in top..bottom {
                            for x in 270..700 {
                                let pixel = first.get_pixel(x, y).0;
                                count +=
                                    usize::from(pixel[0] > 170 && pixel[1] > 170 && pixel[2] > 170);
                            }
                        }
                        count
                    });
                    self.check(
                        "cold-start Inbox renders three task rows",
                        row_text_pixels.iter().all(|pixels| *pixels > 40),
                    );
                    first
                        .save(self.out.join("cold-start-inbox.png"))
                        .expect("save first Inbox frame");
                }
                Err(e) => self.check(&format!("first Inbox frame: {e}"), false),
            }
            self.sleep(cx, 150).await;
            cx.update_window(self.window, |_, window, cx| {
                for _ in 0..2 {
                    window.refresh();
                    window.draw(cx).clear(cx);
                }
                window.render_to_image()
            })
            .expect("window alive")
        } else {
            self.sleep(cx, 150).await;
            cx.update_window(self.window, |_, window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            })
            .expect("window alive");
            self.sleep(cx, 100).await;
            cx.update_window(self.window, |_, window, cx| {
                for _ in 0..2 {
                    window.refresh();
                    window.draw(cx).clear(cx);
                }
                window.render_to_image()
            })
            .expect("window alive")
        };
        match image {
            Ok(image) => image
                .save(self.out.join(format!("{name}.png")))
                .expect("save png"),
            Err(e) => self.check(&format!("screenshot {name}: {e}"), false),
        }
    }
}
