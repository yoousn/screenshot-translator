from PIL import Image

def make_icon_transparent():
    # Load app.ico which has a single 256x256 frame
    ico_path = "d:/Desktop/自制截图/app.ico"
    img = Image.open(ico_path)
    img = img.convert("RGBA")
    
    # Corner pixel defines the background color to replace: (213, 249, 249)
    # To handle compression artifacts, we clear colors close to this cyan shade.
    data = img.getdata()
    new_data = []
    for item in data:
        r, g, b, a = item
        # If green/blue are high and red is moderate (typical cyan background)
        if r > 180 and g > 230 and b > 230:
            new_data.append((255, 255, 255, 0)) # Fully transparent
        else:
            new_data.append(item)
            
    img.putdata(new_data)
    img.save("d:/Desktop/自制截图/app_transparent.ico", format="ICO", sizes=[(256, 256)])
    print("Transparent icon saved successfully.")

if __name__ == "__main__":
    make_icon_transparent()
