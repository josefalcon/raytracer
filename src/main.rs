extern crate cgmath;
extern crate image;

use cgmath::*;
use image::*;

use std::fs::File;
use std::path::Path;

#[derive(Debug)]
struct Ray {
    point: Point3<f32>,
    direction: Vector3<f32>,
}
impl Ray {
    fn new(point: Point3<f32>, direction: Vector3<f32>) -> Ray {
        Ray {
            point: point,
            direction: direction.normalize(),
        }
    }

    fn point_at(&self, t: f32) -> Point3<f32> {
        self.point + (self.direction * t)
    }

    fn through_screen(x: f32, y: f32, width: f32, height: f32, camera_transform: &Matrix4<f32>) -> Ray {
        let screen_point = (
              2.0 * ((x + 0.5) / width) - 1.0,
            -(2.0 * ((y + 0.5) / height) - 1.0),
        );

        let inverse = camera_transform.invert().unwrap();
        let point0 = Vector4::new(
            screen_point.0,
            screen_point.1,
            -1.0,
            1.0);
        let world_point0 = inverse * point0;
        let world_point0 = world_point0 / world_point0.w;

        let point1 = Vector4::new(
            screen_point.0,
            screen_point.1,
            1.0,
            1.0);
        let world_point1 = inverse * point1;
        let world_point1 = world_point1 / world_point1.w;
        let world_dir = (world_point1 - world_point0).normalize();

        Ray { point: Point::from_vec(world_point0.truncate()), direction: world_dir.truncate() }
    }
}

#[derive(Debug, PartialEq)]
struct Sphere {
    center: Point3<f32>,
    radius: f32,
    color: Vector3<f32>,
}
impl Sphere {
    fn new(center: Point3<f32>, radius: f32, color: Vector3<f32>) -> Sphere {
        Sphere {
            center: center,
            radius: radius,
            color: color,
        }
    }
}

trait Intersect {
    fn intersect(&self, ray: &Ray) -> Option<f32>;
}

impl Intersect for Sphere {
    fn intersect(&self, ray: &Ray) -> Option<f32> {
        let l = self.center - ray.point;
        let v = l.dot(ray.direction);
        if v < 0.0 { return None; }

        let d2 = l.dot(l) - v * v;
        let r2 = self.radius * self.radius;
        if d2 > r2 { return None; }

        let d = (r2 - d2).sqrt();
        Some(v - d.min(v + d))
    }
}

struct Camera {
    eye: Point3<f32>,
    center: Point3<f32>,
    up: Vector3<f32>,
    near: f32,
    far: f32,
    fovy: f32,
    aspect_ratio: f32,
}
impl Camera {
    fn new(eye: Point3<f32>, center: Point3<f32>) -> Camera {
        Camera {
            eye: eye,
            center: center,
            up: Vector3::new(0.0, 0.0, 1.0),
            near: 0.1,
            far: 10.0,
            fovy: 1.0,
            aspect_ratio: 1.0,
        }
    }
    fn up(&mut self, up: Vector3<f32>) -> &mut Camera {
        self.up = up;
        self
    }
    fn near(&mut self, near: f32) -> &mut Camera {
        self.near = near;
        self
    }
    fn far(&mut self, far: f32) -> &mut Camera {
        self.far = far;
        self
    }
    fn fovy(&mut self, fovy: f32) -> &mut Camera {
        self.fovy = fovy;
        self
    }
    fn aspect_ratio(&mut self, aspect_ratio: f32) -> &mut Camera {
        self.aspect_ratio = aspect_ratio;
        self
    }
    fn transform(&self) -> Matrix4<f32> {
        let camera = Matrix4::look_at(self.eye, self.center, self.up);
        let projection = perspective(Rad { s: self.fovy }, self.aspect_ratio, self.near, self.far);

        projection * camera
        //
        // let n = (self.eye - self.center).normalize();
        // let u = self.up.cross(n).normalize();
        // let v = n.cross(u).normalize();
        //
        // let height = (self.fovy / 2.0).tan() * 2.0 * self.near;
        // let width = height * self.aspect_ratio;
        // let center = Point3::from_vec(((self.eye - n) * self.near));
    }
}

struct Scene {
    camera: Matrix4<f32>,
    spheres: Vec<Sphere>,
    lights: Vec<Sphere>,
    ambient: Vector3<f32>,
}
impl Scene {
    fn new(camera: Matrix4<f32>) -> Scene {
        Scene {
            camera: camera,
            spheres: vec![],
            lights: vec![],
            ambient: vec3(0.2, 0.2, 0.2),
        }
    }
    fn ambient(&mut self, color: Vector3<f32>) -> &mut Scene {
        self.ambient = color;
        self
    }
    fn add_light(&mut self, center: Point3<f32>, radius: f32, color: Vector3<f32>) -> &mut Scene {
        self.lights.push(Sphere::new(center, radius, color));
        self
    }
    fn add_sphere(&mut self, center: Point3<f32>, radius: f32, color: Vector3<f32>) -> &mut Scene {
        self.spheres.push(Sphere::new(center, radius, color));
        self
    }
    fn trace(&self, ray: &Ray) -> Vector3<f32> {
        let mut closest_sphere = None;
        let mut min_t = std::f32::INFINITY;
        for sphere in &self.spheres {
            match sphere.intersect(ray) {
                Some(t) if t < min_t => {
                    min_t = t;
                    closest_sphere = Some(sphere);
                }
                _ => {}
            }
        }
        if closest_sphere.is_none() { return self.ambient }

        let closest_sphere = closest_sphere.unwrap();

        for light in &self.lights {
            let intersection_point = ray.point_at(min_t);
            let light_direction = (light.center - intersection_point).normalize();
            let light_ray = Ray::new(intersection_point, light_direction);
            for sphere in &self.spheres {
                if sphere == closest_sphere { continue; }
                if sphere.intersect(&light_ray).is_some() {
                    // in shadow...
                    return closest_sphere.color * self.ambient;
                }
            }

            let lambert = (intersection_point - closest_sphere.center).normalize().dot(light_direction).max(0.0);
            let diffuse = vec3(0.5, 0.4, 0.5);
            let illumination = self.ambient + (diffuse * lambert);
            return (closest_sphere.color * illumination);
        }
        return self.ambient;
    }
    fn render(&self, width: u32, height: u32) {
        let mut img = ImageBuffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let ray = Ray::through_screen(x as f32, y as f32, width as f32, height as f32, &self.camera);
                let color = self.trace(&ray);
                img.put_pixel(x, y, image::Rgb([
                    (color[0].min(1.0) * 255.0) as u8,
                    (color[1].min(1.0) * 255.0) as u8,
                    (color[2].min(1.0) * 255.0) as u8]));
            }
        }

        let ref mut fout = File::create(&Path::new("test.png")).unwrap();

        // Write the contents of this image to the Writer in PNG format.
        DynamicImage::ImageRgb8(img).save(fout, image::PNG).unwrap();
    }
}

fn main() {
    let transform = Camera::new(Point3::new(-5.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)).transform();

    let mut scene = Scene::new(transform);
    scene
        .ambient(vec3(0.3, 0.3, 0.3))
        .add_light(Point3::new(-0.5, -2.0, 0.0), 1.0, vec3(1.0, 1.0, 1.0))
        .add_sphere(Point3::new(4.0, 0.0, 3.0), 3.0, vec3(1.0, 0.23, 0.47))
        .add_sphere(Point3::new(1.0, 0.0, 0.0), 1.0, vec3(0.21, 0.1, 0.47));
    scene.render(1024, 1024);
}
