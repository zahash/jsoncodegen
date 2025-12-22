import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.SerializationFeature;

import java.io.IOException;

public class Main {
    public static void main(String[] args) throws IOException {
        ObjectMapper mapper = new ObjectMapper();
        JsonCodeGen.ROOT data = mapper.readValue(System.in, JsonCodeGen.ROOT.class);
        mapper.enable(SerializationFeature.INDENT_OUTPUT);
        mapper.writeValue(System.out, data);
    }
}
