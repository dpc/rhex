# Lint tracked Markdown with the project vocabulary.
vale:
    git ls-files -z '*.md' | xargs -0 -r vale
