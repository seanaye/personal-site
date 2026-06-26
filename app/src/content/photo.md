Hello, this page is currently under construction. I am experimenting with square packing algorithms based on aspect ratio instead of traditional masonry layouts.

I find that the main drawback to using masonry layouts is that they will grow or shrink images to fit a certain column width, which ends up placing more emphasis on portrait orientation photos versus landscape ones.

A square packing algorithm preserves the relative aspect ratio between images so the volume occupied between a 3:2 image vs a 2:3 image is the same. The main tradeoff with this approach is that it can leave gaps in the grid if there is no image which is sized correctly to fit the space.
