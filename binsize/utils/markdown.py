FOOTNOTE_SYMS = ["&ast;", "†", "‡", "§", "¶"]

class Table:
    def __init__(self, title: str, headers: list[str], footnotes: list[str] = []):
        self.title = title
        self.rows: list[str] = []
        self.footnotes: dict[str, str] = {}

        for idx, footnote in enumerate(footnotes, 1):
            if any(f"{{{idx}}}" in cell for cell in headers):
                sym = next(s for s in FOOTNOTE_SYMS if s not in self.footnotes)
                self.footnotes[sym] = footnote.rstrip()
                headers = [cell.replace(f"{{{idx}}}", sym) for cell in headers]

        self.headers = headers

    def add_row(self, row: list[str], footnotes: list[str] = []):
        if len(row) != len(self.headers):
            raise ValueError("Row length does not match header length.")

        for idx, footnote in enumerate(footnotes, 1):
            footnote = footnote.rstrip()
            if any(f"{{{idx}}}" in cell for cell in row):
                syms = [k for k, v in self.footnotes.items() if v == footnote]
                if len(syms) > 0:
                    [sym] = syms # We should never have duplicate values
                else:
                    sym = next(s for s in FOOTNOTE_SYMS if s not in self.footnotes)
                    self.footnotes[sym] = footnote
                row = [cell.replace(f"{{{idx}}}", sym) for cell in row]

        self.rows.append(row)

    def __str__(self):
        widths = [max(len(cell) for cell in col) for col in zip(self.headers, *self.rows)]
        headers = [header.center(width) for header, width in zip(self.headers, widths)]
        rows = [[cell.center(width) for cell, width in zip(row, widths)] for row in self.rows]

        title_str = f"## {self.title}"
        header_str = "| " + " | ".join(headers) + " |"
        separator = "|:" + ":|:".join("-" * len(header) for header in headers) + ":|"
        rows_str = "\n".join("| " + " | ".join(row) + " |" for row in rows)
        footnotes_str = " \\\n".join(f"{sym} *{footnote}*" for sym, footnote in self.footnotes.items())

        return f"{title_str}\n\n{header_str}\n{separator}\n{rows_str}\n\n{footnotes_str}"

def format_urls(urls: dict[str, str]) -> str:
    return "\n".join(f"[{name}]: {url}" for name, url in urls.items())
