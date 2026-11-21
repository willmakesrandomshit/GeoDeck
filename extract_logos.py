"""Extract the Geode loader logos from a user-provided Geode sprite sheet."""

import argparse
from pathlib import Path

from PIL import Image

COLOR_BOUNDS = (456, 0, 684, 229)
FULL_BOUNDS = (0, 326, 324, 650)


def extract_logos(sprite_sheet: Path, output_dir: Path) -> tuple[Path, Path]:
    """Extract the two documented logo regions into output_dir."""
    output_dir.mkdir(parents=True, exist_ok=True)
    with Image.open(sprite_sheet) as sheet:
        if sheet.width < 684 or sheet.height < 650:
            raise ValueError(
                f"Sprite sheet is too small for the expected logo regions: "
                f"{sheet.width}x{sheet.height}."
            )
        color_logo = sheet.crop(COLOR_BOUNDS)
        full_logo = sheet.crop(FULL_BOUNDS)
        color_path = output_dir / "real_geode_logo.png"
        full_path = output_dir / "real_geode_full.png"
        color_logo.save(color_path, format="PNG")
        full_logo.save(full_path, format="PNG")
    return color_path, full_path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sprite_sheet", type=Path, help="Path to the Geode loader logo sprite sheet")
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path(__file__).resolve().parent / "assets",
        help="Directory for extracted PNG files (default: repository assets directory)",
    )
    args = parser.parse_args()

    color_path, full_path = extract_logos(args.sprite_sheet, args.output_dir)
    print(f"Saved {color_path} and {full_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
