// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import java.io.File;
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import javax.xml.XMLConstants;
import javax.xml.transform.stream.StreamSource;
import javax.xml.validation.Schema;
import javax.xml.validation.SchemaFactory;
import org.xml.sax.SAXException;

/**
 * Reference verdicts for the XML Schema comparison: validates every document
 * of a directory with the XML Schema processor of the JDK, the processor the
 * official validators use, and writes for each file whether it is valid.
 *
 * <p>Usage: {@code java SchemaReference.java <documents> <verdicts.json> <schema.xsd>...}
 *
 * <p>Run by {@code tools/regenerate-conformance.sh}. It is a tool of the
 * reference run only and never part of the application.
 */
public final class SchemaReference {
    private static final String DOCUMENT_SUFFIX = ".xml";

    private SchemaReference() {}

    public static void main(String[] arguments) throws Exception {
        File documents = new File(arguments[0]);
        StreamSource[] schemaSources =
                Arrays.stream(arguments)
                        .skip(2)
                        .map(path -> new StreamSource(new File(path)))
                        .toArray(StreamSource[]::new);
        Schema schema =
                SchemaFactory.newInstance(XMLConstants.W3C_XML_SCHEMA_NS_URI)
                        .newSchema(schemaSources);

        File[] files = documents.listFiles((directory, name) -> name.endsWith(DOCUMENT_SUFFIX));
        Arrays.sort(files);
        try (PrintWriter out = new PrintWriter(arguments[1], StandardCharsets.UTF_8)) {
            out.print("{");
            for (int index = 0; index < files.length; index++) {
                out.print(index == 0 ? "" : ",");
                out.print("\"" + files[index].getName() + "\":" + isValid(schema, files[index]));
            }
            out.println("}");
        }
    }

    private static boolean isValid(Schema schema, File document) throws java.io.IOException {
        try {
            schema.newValidator().validate(new StreamSource(document));
            return true;
        } catch (SAXException invalid) {
            return false;
        }
    }
}
