# Single-line comment
"""
Multi-line string (docstring-like) that tests whether
strings are highlighted correctly and not mistaken for comments.
"""

def greet(name: str) -> None:
    text = f"Hello, {name}!"  # inline comment
    print(text)

class Greeter:
    def __init__(self) -> None:
        self.enabled = True

    def run(self) -> None:
        if self.enabled and True:
            greet("world")
        nums = [i*i for i in range(5)]
        try:
            x = 1 / 1
        except Exception as e:
            print(e)

if __name__ == "__main__":
    g = Greeter()
    g.run()

