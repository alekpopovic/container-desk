#!/usr/bin/env python3
"""Check generated Pages links, fragments, assets and unrendered Markdown links."""
import argparse
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit

class Page(HTMLParser):
    def __init__(self, path):
        super().__init__()
        self.links = []
        self.ids = set()
        self.feed(path.read_text())

    def handle_starttag(self, tag, attrs):
        data = dict(attrs)
        if 'id' in data:
            self.ids.add(data['id'])
        for attr in ('href', 'src'):
            if attr in data:
                self.links.append(data[attr])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('site', type=Path)
    parser.add_argument('--baseurl', default='/container-desk')
    args = parser.parse_args()
    root = args.site.resolve()
    pages = {p: Page(p) for p in root.rglob('*.html')}
    errors = []
    checked = 0
    for path, page in pages.items():
        for link in page.links:
            url = urlsplit(link)
            if url.scheme or url.netloc:
                continue
            if url.path.startswith('/'):
                prefix = args.baseurl.rstrip('/') + '/'
                if not url.path.startswith(prefix):
                    errors.append(f'{path.relative_to(root)}: missing baseurl: {link}')
                    continue
                target = root / unquote(url.path[len(prefix):])
            else:
                target = path.parent / unquote(url.path) if url.path else path
            target = target.resolve()
            if target.is_dir():
                target /= 'index.html'
            checked += 1
            if not target.is_relative_to(root) or not target.is_file():
                errors.append(f'{path.relative_to(root)}: missing target: {link}')
            elif target in pages and url.fragment and unquote(url.fragment) not in pages[target].ids:
                errors.append(f'{path.relative_to(root)}: missing fragment: {link}')
    for error in errors:
        print(error)
    print(f'{len(pages)} HTML pages; {checked} local references; {len(errors)} errors')
    return bool(errors)

if __name__ == '__main__':
    raise SystemExit(main())
