import tkinter
from ex import Url, Browser

if __name__ == "__main__":
    import sys

    Browser().load(Url(sys.argv[1]))
    tkinter.mainloop()
