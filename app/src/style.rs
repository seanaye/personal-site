use grid::GridContent;
use photogrid::PhotoGrid;

pub trait CssStyle<T> {
    fn style(&self, marker: T) -> String;
}

pub struct GridElemClass;
impl<T> CssStyle<GridElemClass> for GridContent<T> {
    fn style(&self, _: GridElemClass) -> String {
        let (size, origin) = self.grid_area();
        let row_start = origin.y + 1;
        let col_start = origin.x + 1;
        let row_span = size.height;
        let col_span = size.width;
        format!(
            "col-start-{col_start} row-start-{row_start} col-span-{col_span} row-span-{row_span}"
        )
    }
}
pub struct GridElemStyle;
impl<T> CssStyle<GridElemStyle> for GridContent<T> {
    fn style(&self, _: GridElemStyle) -> String {
        let (size, _) = self.grid_area();

        format!("aspect-ratio: {}/{};", size.width, size.height)
    }
}

pub struct GridOuterClass;
impl<T> CssStyle<GridOuterClass> for PhotoGrid<T> {
    fn style(&self, _: GridOuterClass) -> String {
        let width = self.width;

        let display = match width {
            12 => "hidden 2xl:grid grid-cols-12",
            8 => "hidden lg:max-2xl:grid grid-cols-8",
            5 => "hidden md:max-lg:grid grid-cols-5",
            4 => "hidden sm:max-md:grid grid-cols-4",
            3 => "grid sm:hidden grid-cols-3",
            _ => panic!("unmatched grid width"),
        };

        format!("w-full {display}")
    }
}
