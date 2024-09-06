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

[![Crates.io](https://img.shields.io/crates/v/jcg.svg)](https://crates.io/crates/jcg)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

</div>

JSONCodeGen is a versatile tool designed to convert JSON files into code for various programming languages, facilitating the creation of classes, structs, or equivalent data structures for serialization and deserialization.

## 🚀 Installation

To use JSONCodeGen, download the binary executable for your platform from the [Releases](https://github.com/zahash/jsoncodegen/releases) page on GitHub. Place the executable in your desired directory and ensure it's included in your system's PATH environment variable.

( or )

Install it using cargo

```sh
cargo install jcg
```

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
jcg --filepath sample.json java
```

#### Output

```java
// Books.java
import com.fasterxml.jackson.annotation.*;
public class Books {
    private String author;
    private List<String> genres;
    private String title;
    public String getAuthor() { return author; }
    public void setAuthor(String value) { this.author = value; }
    public List<String> getGenres() { return genres; }
    public void setGenres(List<String> value) { this.genres = value; }
    public String getTitle() { return title; }
    public void setTitle(String value) { this.title = value; }
}
// Library.java
import com.fasterxml.jackson.annotation.*;
public class Library {
    private List<Books> books;
    private String name;
    public List<Books> getBooks() { return books; }
    public void setBooks(List<Books> value) { this.books = value; }
    public String getName() { return name; }
    public void setName(String value) { this.name = value; }
}
// Root.java
import com.fasterxml.jackson.annotation.*;
public class Root {
    private Library library;
    public Library getLibrary() { return library; }
    public void setLibrary(Library value) { this.library = value; }
}
```

### Complex Example

```json
{
    "items": ["one", 2, 3.0],
    "point": {
        "x": 1,
        "y": 2.0
    }
}
```

```java
// Root.java
import com.fasterxml.jackson.annotation.*;
public class Root {
    private List<Items> items;
    private Point point;
    @JsonProperty("itemsこんにちは")
    public List<Items> getItems() { return items; }
    @JsonProperty("itemsこんにちは")
    public void setItems(List<Items> value) { this.items = value; }
    public Point getPoint() { return point; }
    public void setPoint(Point value) { this.point = value; }
}
// Point.java
import com.fasterxml.jackson.annotation.*;
public class Point {
    private Long x;
    private Double y;
    public Long getX() { return x; }
    public void setX(Long value) { this.x = value; }
    public Double getY() { return y; }
    public void setY(Double value) { this.y = value; }
}
// Items.java
import java.io.IOException;
import com.fasterxml.jackson.core.*;
import com.fasterxml.jackson.databind.*;
import com.fasterxml.jackson.databind.annotation.*;
@JsonSerialize(using = Items.Serializer.class)
@JsonDeserialize(using = Items.Deserializer.class)
public class Items {
    public String strVal;
    public Long longVal;
    public Double doubleVal;
    static class Serializer extends JsonSerializer<Items> {
        @Override public void serialize(Items value, JsonGenerator generator, SerializerProvider serializer) throws IOException {
            if (value.strVal != null) { generator.writeObject(value.strVal); return; }
            if (value.longVal != null) { generator.writeObject(value.longVal); return; }
            if (value.doubleVal != null) { generator.writeObject(value.doubleVal); return; }
            generator.writeNull();
        }
    }
    static class Deserializer extends JsonDeserializer<Items> {
        @Override public Items deserialize(JsonParser parser, DeserializationContext ctx) throws IOException {
            Items value = new Items();
            switch (parser.currentToken()) {
            case VALUE_NULL: break;
            case VALUE_STRING: value.strVal = parser.readValueAs(String.class); break;
            case VALUE_NUMBER_INT: value.longVal = parser.readValueAs(Long.class); break;
            case VALUE_NUMBER_FLOAT: value.doubleVal = parser.readValueAs(Double.class); break;
            default: throw new IOException("Cannot deserialize Items");
            }
            return value;
        }
    }
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
