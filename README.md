<div align="center">

<pre>
     ██╗███████╗ ██████╗ ███╗   ██╗     ██████╗ ██████╗ ██████╗ ███████╗ ██████╗ ███████╗███╗   ██╗
     ██║██╔════╝██╔═══██╗████╗  ██║    ██╔════╝██╔═══██╗██╔══██╗██╔════╝██╔════╝ ██╔════╝████╗  ██║
     ██║███████╗██║   ██║██╔██╗ ██║    ██║     ██║   ██║██║  ██║█████╗  ██║  ███╗█████╗  ██╔██╗ ██║
██   ██║╚════██║██║   ██║██║╚██╗██║    ██║     ██║   ██║██║  ██║██╔══╝  ██║   ██║██╔══╝  ██║╚██╗██║
╚█████╔╝███████║╚██████╔╝██║ ╚████║    ╚██████╗╚██████╔╝██████╔╝███████╗╚██████╔╝███████╗██║ ╚████║
 ╚════╝ ╚══════╝ ╚═════╝ ╚═╝  ╚═══╝     ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝ ╚═════╝ ╚══════╝╚═╝  ╚═══╝
---------------------------------------------------------------------------------------------------
A tool for converting JSON files into code for multiple programming languages. Made with ❤️ using 🦀
</pre>

[![Crates.io](https://img.shields.io/crates/v/jsoncodegen.svg)](https://crates.io/crates/jsoncodegen)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

</div>

JSONCodeGen is a versatile tool designed to convert JSON files into code for various programming languages, facilitating the creation of classes, structs, or equivalent data structures for serialization and deserialization.

## 🚀 Installation

To use JSONCodeGen, download the binary executable for your platform from the Releases page on GitHub. Place the executable in your desired directory and ensure it's included in your system's PATH environment variable.

## 🧑‍💻 Usage

### 1. Create a JSON File

Prepare a JSON file containing the data structure you want to convert into code. This JSON will be the source for generating the schema and corresponding code.

#### Example JSON file

```json
{
  "library": {
    "name": "City Library",
    "books": [
      {
        "title": "1984",
        "author": "George Orwell",
        "genres": ["Dystopian", "Political Fiction"]
      },
      {
        "title": "To Kill a Mockingbird",
        "author": "Harper Lee",
        "genres": ["Classic", "Historical Fiction"]
      }
    ]
  }
}
```

### 2. Run JSONCodeGen

Run the JSONCodeGen executable in the same directory as your JSON file or specify the path to the file. You can specify the language subcommand (like java, python, cpp) along with language-specific options. use --help to see all available options.

```sh
jsoncodegen --filepath sample.json java --attribute-access-modifier private --getters --setters
```

this is the output

```java
public class books {
    private  String author;
    private  List<String> genres;
    private  String title;
    public String getauthor() { return author; }
    public List<String> getgenres() { return genres; }
    public String gettitle() { return title; }
    public void setauthor(String author) { this.author = author; }
    public void setgenres(List<String> genres) { this.genres = genres; }
    public void settitle(String title) { this.title = title; }
}
public class library {
    private  List<books> books;
    private  String name;
    public List<books> getbooks() { return books; }
    public String getname() { return name; }
    public void setbooks(List<books> books) { this.books = books; }
    public void setname(String name) { this.name = name; }
}
public class Root {
    private  library library;
    public library getlibrary() { return library; }
    public void setlibrary(library library) { this.library = library; }
}
```

## 🌟 Connect with Us

M. Zahash – zahash.z@gmail.com

Distributed under the MIT license. See `LICENSE` for more information.

[https://github.com/zahash/](https://github.com/zahash/)

## 🤝 Contribute to JSONCodeGen!

1. Fork it (<https://github.com/zahash/jsoncodegen/fork>)
2. Create your feature branch (`git checkout -b feature/fooBar`)
3. Commit your changes (`git commit -am 'Add some fooBar'`)
4. Push to the branch (`git push origin feature/fooBar`)
5. Create a new Pull Request

❤️ Show Some Love!

If you find JSONCodeGen helpful, consider giving it a star on GitHub! Your support encourages continuous improvement and development.
