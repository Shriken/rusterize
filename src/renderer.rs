use std::cmp::Ordering::Equal;
use std::error;
use std::f64;
use std::mem;

use pixel;
use pixel::Pixel;
use screen::Screen;
use texture::Texture;
use types::*;


pub struct Renderer<S>
    where S: Screen
{
    screen: S,
    texture: Texture,

    transform: Transform,
    color: Pixel,

    light: Point,
}

#[allow(dead_code)]
impl<S> Renderer<S>
    where S: Screen
{
    pub fn new(screen: S) -> Renderer<S> {
        let w = screen.width();
        let h = screen.height();

        Renderer {
            screen: screen,
            texture: Texture::new(w, h),

            transform: Transform::identity(),
            color: pixel::WHITE,

            light: pt![0., 0., 0.],
        }
    }

    pub fn draw_point(&mut self, p: Point) {
        let p = p * self.transform;
        let d = 7;
        for row in 0 .. d {
            self.texture.set_row(
                p.x as PixCoord - d / 2,
                p.x as PixCoord + d / 2,
                p.y as PixCoord + row - d / 2,
                -p.z,
                -p.z,
                self.color
            );
        }
    }

    fn draw_point_with_transform(&mut self, p: Point, transform: Transform) {
        let old_transform = self.transform;
        self.transform = transform;
        self.draw_point(p);
        self.transform = old_transform;
    }

    pub fn draw_line(&mut self, p1: Point, p2: Point) {
        let p1 = p1 * self.transform;
        let p2 = p2 * self.transform;
        let p1x = p1.x as PixCoord;
        let p1y = p1.y as PixCoord;
        let p2x = p2.x as PixCoord;
        let p2y = p2.y as PixCoord;

        let dx = p2.x as i64 - p1.x as i64;
        let dy = p2.y as i64 - p1.y as i64;
        let adx = if dx >= 0 { dx } else { -dx };
        let ady = if dy >= 0 { dy } else { -dy };

        let x_step = if p2x > p1x { 1 } else { -1 };
        let y_step = if p2y > p1y { 1 } else { -1 };
        let mut x = p1x;
        let mut y = p1y;
        let mut error: i64 = 0;
        loop {
            if adx >= ady {
                if 2 * error > adx {
                    y += y_step;
                    error -= adx;
                }
                error += ady;
            } else {
                if 2 * error > ady {
                    x += x_step;
                    error -= ady;
                }
                error += adx;
            }

            // FIXME: Do depth lerping.
            self.texture.set_pixel(
                x,
                y,
                f64::INFINITY,
                self.color
            );

            if adx >= ady {
                if x == p2x { break }
                else { x += x_step }
            } else {
                if y == p2y { break }
                else { y += y_step }
            }
        }
    }

    fn draw_line_with_transform(
        &mut self,
        p1: Point,
        p2: Point,
        transform: Transform
    ) {
        let old_transform = self.transform;
        self.transform = transform;
        self.draw_line(p1, p2);
        self.transform = old_transform;
    }

    pub fn fill_triangle(&mut self, t: Triangle) {
        let centroid = (t.p1 + t.p2 + t.p3) * (1. / 3.);
        let ct = t * self.transform;
        if ct.normal().dot(centroid) >= 0. { return }

        let mut pts = ct.to_arr();
        pts.sort_by(
            |p1, p2|
            p1.y.partial_cmp(&p2.y)
                .unwrap_or(Equal)
        );
        let (top, middle, bot) = (pts[0], pts[1], pts[2]);

        // Compute color of triangle based on light.
        let old_color = self.color;
        self.color = pixel::WHITE; {
            let light_dir = (self.light - centroid).normalized();
            let light_mag = light_dir.dot(t.normal()).max(0.);
            let (r, g, b) = self.color;
            (
                (r as f64 * light_mag) as u8,
                (g as f64 * light_mag) as u8,
                (b as f64 * light_mag) as u8
            )
        };

        if      top.y == middle.y { self.fill_top_flat_triangle(ct); }
        else if middle.y == bot.y { self.fill_bottom_flat_triangle(ct); }
        else {
            let dy_middle = (middle.y - top.y) as f64;
            let dy_bot = (bot.y - top.y) as f64;
            let dx_bot = (bot.x - top.x) as f64;
            let dz_bot = (bot.z - top.z) as f64;

            let v4 = pt![
                top.x + ((dy_middle / dy_bot) * dx_bot) as Coord,
                middle.y,
                top.z + ((dy_middle / dy_bot) * dz_bot) as Coord
            ];
            self.fill_bottom_flat_triangle(trigon![top, middle, v4]);
            self.fill_top_flat_triangle(trigon![middle, v4, bot]);
        }
        self.color = old_color;
    }

    fn fill_bottom_flat_triangle(&mut self, t: Triangle) {
        let (top, mut left, mut right) = t.to_tuple();
        if left.x > right.x { mem::swap(&mut left, &mut right) }
        let invslope1 = (left.x - top.x)  / (left.y - top.y);
        let invslope2 = (right.x - top.x) / (right.y - top.y);
        let mut curx1 = top.x;
        let mut curx2 = top.x;

        for y in top.y as PixCoord .. left.y as PixCoord {
            let t       = (y as Coord - top.y) / (left.y - top.y);
            let z_left  = left.z  * t + top.z * (1. - t);
            let z_right = right.z * t + top.z * (1. - t);

            self.texture.set_row(
                curx1 as PixCoord,
                curx2 as PixCoord,
                y,
                -z_left,
                -z_right,
                self.color
            );
            curx1 += invslope1;
            curx2 += invslope2;
        }

        let t_right = (left.y - top.y) / (right.y - top.y);
        let z_right = right.z * t_right + top.z * (1. - t_right);
        self.texture.set_row(
            left.x  as PixCoord,
            right.x as PixCoord,
            left.y  as PixCoord,
            -left.z,
            -z_right,
            self.color
        );
    }

    fn fill_top_flat_triangle(&mut self, t: Triangle) {
        let (mut left, mut right, bot) = t.to_tuple();
        if left.x > right.x { mem::swap(&mut left, &mut right) }
        let invslope1 = (bot.x - left.x)  / (bot.y - left.y);
        let invslope2 = (bot.x - right.x) / (bot.y - right.y);
        let mut curx1 = left.x;
        let mut curx2 = right.x;

        for y in left.y as PixCoord .. bot.y as PixCoord + 1 {
            let t       = (y as Coord - left.y) / (bot.y - left.y);
            let z_left  = t + bot.z * t + left.z  * (1. - t);
            let z_right = t + bot.z * t + right.z * (1. - t);

            self.texture.set_row(
                curx1 as PixCoord,
                curx2 as PixCoord,
                y,
                -z_left,
                -z_right,
                self.color
            );
            curx1 += invslope1;
            curx2 += invslope2;
        }
    }

    pub fn clear(&mut self) {
        self.texture.clear();
    }

    pub fn display(&mut self) -> Result<(), Box<error::Error>> {
        self.screen.display_texture(&self.texture)
    }


    pub fn set_transform(&mut self, t: Transform) {
        self.transform = t;
    }

    pub fn clear_transform(&mut self) {
        self.transform = Transform::identity();
    }

    pub fn translate(&mut self, p: Point) {
        self.transform = Transform::translate(p) * self.transform;
    }

    pub fn rotate_x(&mut self, theta: f64) {
        self.transform = Transform::rotate_x(theta) * self.transform;
    }

    pub fn rotate_y(&mut self, theta: f64) {
        self.transform = Transform::rotate_y(theta) * self.transform;
    }

    pub fn rotate_z(&mut self, theta: f64) {
        self.transform = Transform::rotate_z(theta) * self.transform;
    }

    pub fn scale(&mut self, x: f64, y: f64, z: f64) {
        self.transform = Transform::scale(x, y, z) * self.transform;
    }

    pub fn perspective(&mut self) {
        self.transform = Transform::perspective() * self.transform;
    }


    pub fn set_color(&mut self, color: Pixel) { self.color = color; }

    pub fn set_light_pos(&mut self, pos: Point) { self.light = pos; }
}
