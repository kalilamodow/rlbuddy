# makes the table of contents in the readme yay

from dataclasses import dataclass


def name_to_id(name: str) -> str:
    return "-".join(p.lower() for p in name.split(" ")).replace("/", "")


@dataclass
class MDHeading:
    name: str
    id: str
    level: int

    def to_string(self) -> str:
        spaces = "    " * self.level
        href = f"#{self.id}"

        return f"{spaces}- [{self.name}]({href})"


def main():
    with open("README.md", "r") as readme_file:
        readme_content = readme_file.readlines()

    headings: list[MDHeading] = []
    for line in readme_content:
        if not line.startswith("#"):
            continue

        num_pounds = len(line.split(" ")[0])
        # ignore main header and really subheaders
        if num_pounds == 1 or num_pounds > 3:
            continue
        # because we ignore main header, normalize -1. also # would be 0 so -1
        level = num_pounds - 2

        name = line[(num_pounds + 1) :]
        name = name.strip()
        headings.append(MDHeading(name, name_to_id(name), level))

    print(headings)
    text = "\n".join(h.to_string() for h in headings)
    print(text)


if __name__ == "__main__":
    main()
